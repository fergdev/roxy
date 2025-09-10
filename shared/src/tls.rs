use std::{error::Error, sync::Arc};

use hyper_util::rt::tokio::WithHyperIo;
use rustls::{
    ClientConfig, RootCertStore, ServerConfig, SupportedCipherSuite,
    crypto::CryptoProvider,
    pki_types::ServerName,
    sign::CertifiedKey,
    version::{TLS12, TLS13},
};
use tokio::net::TcpStream;
use tracing::{error, trace};

use crate::{
    RoxyCA,
    alpn::AlpnProtocol,
    cert::{
        ClientTlsConnectionData, LoggingResolvesClientCert, LoggingResolvesServerCert,
        LoggingServerVerifier,
    },
    crypto::init_crypto,
    http::{HttpEmitter, HttpError, HttpEvent},
    io::IOTypeNotSend,
};

#[derive(Debug, Clone)]
pub struct TlsConfig {
    crypto_provider: Arc<CryptoProvider>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        init_crypto();
        let provider = rustls::crypto::aws_lc_rs::default_provider();
        TlsConfig::from_provider(provider)
    }
}

pub struct RustlsClientConfig {
    pub cert_logger: Arc<LoggingServerVerifier>,
    pub resolver: Arc<LoggingResolvesClientCert>,
    pub client_config: ClientConfig,
}

pub struct RustlsServerConfig {
    pub resolver: Arc<LoggingResolvesServerCert>,
    pub server_config: ServerConfig,
}

impl TlsConfig {
    pub fn from_provider(provider: CryptoProvider) -> Self {
        let crypto_provider = CryptoProvider {
            cipher_suites: provider.cipher_suites.clone(),
            kx_groups: provider.kx_groups.clone(),
            signature_verification_algorithms: provider.signature_verification_algorithms,
            secure_random: provider.secure_random,
            key_provider: provider.key_provider,
        };
        Self {
            crypto_provider: Arc::new(crypto_provider),
        }
    }

    pub fn crypto_provider(&self) -> Arc<CryptoProvider> {
        self.crypto_provider.clone()
    }

    pub fn rustls_client_config(&self, root_store: Arc<RootCertStore>) -> RustlsClientConfig {
        let cert_logger = Arc::new(LoggingServerVerifier::with_root_store_provider(
            root_store.clone(),
            self.crypto_provider.clone(),
        ));
        let resolver = Arc::new(LoggingResolvesClientCert::default());

        let client_config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(cert_logger.clone())
            .with_client_cert_resolver(resolver.clone());
        RustlsClientConfig {
            cert_logger,
            resolver,
            client_config,
        }
    }

    pub fn rustls_server_config(
        &self,
        certified_key: CertifiedKey,
    ) -> Result<RustlsServerConfig, Box<dyn Error>> {
        let versions = self
            .crypto_provider
            .cipher_suites
            .iter()
            .map(|cs| match cs {
                SupportedCipherSuite::Tls12(_) => &TLS12,
                SupportedCipherSuite::Tls13(_) => &TLS13,
            })
            .collect::<Vec<_>>();
        let resolver = Arc::new(LoggingResolvesServerCert::new(certified_key));
        let server_config = ServerConfig::builder_with_provider(self.crypto_provider.clone())
            .with_protocol_versions(versions.as_slice())?
            .with_no_client_auth()
            .with_cert_resolver(resolver.clone());

        Ok(RustlsServerConfig {
            resolver,
            server_config,
        })
    }
}

#[derive(Debug)]
pub enum TlsVersion {
    V2,
    V3,
}

pub async fn client_tls(
    server_name: ServerName<'static>,
    stream: WithHyperIo<TcpStream>,
    alpn_protocols: Vec<Vec<u8>>,
    root_store: Arc<RootCertStore>,
    emitter: &dyn HttpEmitter,
    tls_config: &TlsConfig,
) -> Result<(Box<dyn RTls>, AlpnProtocol), HttpError> {
    let RustlsClientConfig {
        cert_logger,
        resolver: _,
        mut client_config,
    } = tls_config.rustls_client_config(root_store);

    client_config.enable_sni = true;
    client_config.alpn_protocols = alpn_protocols;

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    emitter.emit(HttpEvent::ClientTlsHandshake);
    let tls = connector
        .connect(server_name, stream)
        .await
        .map_err(|err| HttpError::TlsError(std::io::Error::other(format!("{err}"))))?;

    trace!("TLS connected");
    let tls_conn_data: ClientTlsConnectionData = tls.get_ref().1.into();
    let alpn = tls_conn_data.alpn.clone();
    let server_verification = cert_logger
        .certs
        .lock()
        .map_err(|e| {
            error!("{e}");
            HttpError::TlsError(std::io::Error::other(format!(
                "Lock server verification {e}"
            )))
        })?
        .to_owned();
    emitter.emit(HttpEvent::ClientTlsConn(tls_conn_data, server_verification));

    Ok((Box::new(IOTypeNotSend::new_raw(tls)), alpn))
}

pub trait RTls: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static {}

impl RTls for IOTypeNotSend<tokio_rustls::client::TlsStream<WithHyperIo<TcpStream>>> {}
impl RTls for IOTypeNotSend<tokio_native_tls::TlsStream<WithHyperIo<TcpStream>>> {}

pub async fn client_tls_native(
    server_name: ServerName<'static>,
    stream: WithHyperIo<TcpStream>,
    alpn_protocols: &[&str],
    root_store: RoxyCA,
    emitter: &dyn HttpEmitter,
) -> Result<(Box<dyn RTls>, AlpnProtocol), HttpError> {
    trace!("TLS native conn");
    let cert = native_tls::Certificate::from_der(&root_store.inner.ca_der)
        .map_err(std::io::Error::other)?;

    emitter.emit(HttpEvent::ServerTlsConnInitiated);
    let native_conn = native_tls::TlsConnector::builder()
        .request_alpns(alpn_protocols)
        .min_protocol_version(None)
        .add_root_certificate(cert)
        .build()
        .map_err(std::io::Error::other)?;

    let native_conn = tokio_native_tls::TlsConnector::from(native_conn);

    trace!("Connecting");
    let tls = native_conn
        .connect(&server_name.to_str(), stream)
        .await
        .map_err(|err| HttpError::TlsError(std::io::Error::other(format!("{err}"))))?;

    let alpn = tls
        .get_ref()
        .negotiated_alpn()
        .unwrap_or(None)
        .as_ref()
        .map_or(AlpnProtocol::None, |v| {
            AlpnProtocol::from_bytes(v.as_slice())
        });

    trace!("TLS connected");
    trace!("TLS end");
    Ok((Box::new(IOTypeNotSend::new_raw(tls)), alpn))
}
