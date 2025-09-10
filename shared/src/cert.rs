use bytes::Bytes;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use std::sync::{Arc, Mutex};

use crate::alpn::AlpnProtocol;
use rustls::client::{EchStatus, ResolvesClientCert, WebPkiServerVerifier};
use rustls::pki_types::ServerName;
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{ClientHello, ResolvesServerCert, WebPkiClientVerifier};
use rustls::sign::CertifiedKey;
use rustls::{
    ClientConnection, ProtocolVersion, RootCertStore, ServerConnection, SignatureScheme,
    SupportedCipherSuite, pki_types::*,
};
use tracing::trace;

#[derive(Debug, Default, Clone)]
pub struct ServerVerificationCapture {
    pub cert: Option<VerifyServerCert>,
    pub tls: TlsVerify,
}

#[derive(Debug, Default, Clone)]
pub struct ClientVerificationCapture {
    pub cert: Option<VerifyClientCert>,
    pub tls: TlsVerify,
}

#[derive(Debug, Clone)]
pub struct VerifyClientCert {
    pub end_endity: Bytes,
    pub intermediates: Vec<Bytes>,
    pub now: UnixTime,
    pub error: Option<rustls::Error>,
}

#[derive(Debug, Clone)]
pub struct VerifyServerCert {
    pub end_entity: Bytes,
    pub intermediates: Vec<Bytes>,
    pub server_name: ServerName<'static>,
    pub ocsp_response: Bytes,
    pub now: UnixTime,
    pub error: Option<rustls::Error>,
}

#[derive(Debug, Clone, Default)]
pub enum TlsVerify {
    Tls13(TlsCapture),
    Tls12(TlsCapture),
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub struct TlsCapture {
    pub message: Bytes,
    pub cert: Bytes,
    pub dss: rustls::DigitallySignedStruct,
    pub error: Option<rustls::Error>,
}

#[derive(Default, Debug, Clone)]
pub struct ServerTlsConnectionData {
    pub protocol_version: Option<ProtocolVersion>,
    pub cipher_suite: Option<SupportedCipherSuite>,
    pub sni: Option<String>,
    pub key_exchange_group: Option<String>,
    pub alpn: AlpnProtocol,
}

impl From<&ServerConnection> for ServerTlsConnectionData {
    fn from(tls_session: &ServerConnection) -> Self {
        let tls_version = tls_session.protocol_version();
        let cipher_suite = tls_session.negotiated_cipher_suite();
        let sni = tls_session.server_name();
        let alpn_bytes = tls_session.alpn_protocol();
        let key_exchange_group = tls_session.negotiated_key_exchange_group();
        let alpn = AlpnProtocol::from_bytes_opt(alpn_bytes);

        ServerTlsConnectionData {
            protocol_version: tls_version,
            cipher_suite,
            sni: sni.map(String::from),
            key_exchange_group: key_exchange_group.map(|v| format!("{v:?}")),
            alpn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientTlsConnectionData {
    pub protocol_version: Option<ProtocolVersion>,
    pub cipher_suite: Option<SupportedCipherSuite>,
    pub ech_status: EchStatus,
    pub key_exchange_group: Option<String>,
    pub alpn: AlpnProtocol,
}

impl From<&ClientConnection> for ClientTlsConnectionData {
    fn from(tls_session: &ClientConnection) -> Self {
        let tls_version = tls_session.protocol_version();
        let cipher_suite = tls_session.negotiated_cipher_suite();
        let ech_status = tls_session.ech_status();
        let alpn_bytes = tls_session.alpn_protocol();
        let key_exchange_group = tls_session.negotiated_key_exchange_group();
        let alpn = AlpnProtocol::from_bytes_opt(alpn_bytes);

        ClientTlsConnectionData {
            protocol_version: tls_version,
            cipher_suite,
            ech_status,
            key_exchange_group: key_exchange_group.map(|v| format!("{v:?}")),
            alpn,
        }
    }
}

#[derive(Debug)]
pub struct LoggingClientVerifier {
    pub certs: std::sync::Mutex<ClientVerificationCapture>,
    name: Vec<rustls::DistinguishedName>,
    inner: Option<Arc<dyn ClientCertVerifier>>,
}

impl LoggingClientVerifier {
    pub fn new() -> Self {
        LoggingClientVerifier {
            certs: std::sync::Mutex::new(ClientVerificationCapture::default()),
            name: vec![],
            inner: None,
        }
    }

    pub fn with_inner(root_store: Arc<RootCertStore>) -> Self {
        // let provider = rustls::crypto::aws_lc_rs::default_provider();
        let provider = rustls::crypto::ring::default_provider();

        let inner = WebPkiClientVerifier::builder_with_provider(root_store, Arc::new(provider))
            .build()
            .map(Some)
            .unwrap_or(None);

        LoggingClientVerifier {
            certs: std::sync::Mutex::new(ClientVerificationCapture::default()),
            name: vec![],
            inner,
        }
    }
}

impl Default for LoggingClientVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientCertVerifier for LoggingClientVerifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &self.name
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.cert = Some(VerifyClientCert {
            end_endity: end_entity.to_vec().into(),
            intermediates: intermediates.iter().map(|i| i.to_vec().into()).collect(),
            now,
            error: None,
        });

        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        let verification_result = self
            .inner
            .as_ref()
            .map(|v| v.verify_tls12_signature(message, cert, dss))
            .unwrap_or(Ok(HandshakeSignatureValid::assertion()));

        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.tls = TlsVerify::Tls12(TlsCapture {
            message: message.to_vec().into(),
            cert: cert.to_vec().into(),
            dss: dss.clone(),
            error: verification_result.as_ref().err().cloned(),
        });

        verification_result
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        trace!("verify_tls13_signature");
        let verification_result = self
            .inner
            .as_ref()
            .map(|v| v.verify_tls13_signature(message, cert, dss))
            .unwrap_or(Ok(HandshakeSignatureValid::assertion()));

        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.tls = TlsVerify::Tls13(TlsCapture {
            message: message.to_vec().into(),
            cert: cert.to_vec().into(),
            dss: dss.clone(),
            error: verification_result.as_ref().err().cloned(),
        });
        verification_result
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner
            .as_ref()
            .map(|f| f.supported_verify_schemes())
            .unwrap_or(vec![
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
            ])
    }
}

#[derive(Debug)]
pub struct LoggingServerVerifier {
    pub certs: std::sync::Mutex<ServerVerificationCapture>,
    inner: Option<Arc<WebPkiServerVerifier>>,
}

impl LoggingServerVerifier {
    pub fn new() -> Self {
        LoggingServerVerifier {
            certs: std::sync::Mutex::new(ServerVerificationCapture::default()),
            inner: None,
        }
    }

    pub fn with_root_store_provider(
        root_store: Arc<RootCertStore>,
        crypto_provider: Arc<CryptoProvider>,
    ) -> Self {
        let inner = WebPkiServerVerifier::builder_with_provider(root_store, crypto_provider)
            .build()
            .map(Some)
            .unwrap_or(None);
        LoggingServerVerifier {
            certs: std::sync::Mutex::new(ServerVerificationCapture::default()),
            inner,
        }
    }
}

impl Default for LoggingServerVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCertVerifier for LoggingServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        trace!("Verifying server certificate for: {:?}", server_name);

        let res = self
            .inner
            .as_ref()
            .map(|v| {
                v.verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
            })
            .unwrap_or(Ok(ServerCertVerified::assertion()));

        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.cert = Some(VerifyServerCert {
            end_entity: end_entity.to_vec().into(),
            intermediates: intermediates.iter().map(|i| i.to_vec().into()).collect(),
            server_name: server_name.to_owned(),
            ocsp_response: ocsp_response.to_vec().into(),
            now,
            error: res.as_ref().err().cloned(),
        });

        res
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        let res = self
            .inner
            .as_ref()
            .map(|v| v.verify_tls12_signature(message, cert, dss))
            .unwrap_or(Ok(HandshakeSignatureValid::assertion()));

        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.tls = TlsVerify::Tls12(TlsCapture {
            message: message.to_vec().into(),
            cert: cert.to_vec().into(),
            dss: dss.clone(),
            error: res.as_ref().err().cloned(),
        });

        res
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        let res = self
            .inner
            .as_ref()
            .map(|v| v.verify_tls13_signature(message, cert, dss))
            .unwrap_or(Ok(HandshakeSignatureValid::assertion()));

        let mut guard = self
            .certs
            .lock()
            .map_err(|e| rustls::Error::General(format!("Failed to gain lock on certs {e}")))?;

        guard.tls = TlsVerify::Tls13(TlsCapture {
            message: message.to_vec().into(),
            cert: cert.to_vec().into(),
            dss: dss.clone(),
            error: res.as_ref().err().cloned(),
        });
        res
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner
            .as_ref()
            .map(|f| f.supported_verify_schemes())
            .unwrap_or(vec![
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
            ])
    }
}

#[derive(Debug, Clone)]
pub struct CapturedClientHello {
    pub data: String,
}

impl From<ClientHello<'_>> for CapturedClientHello {
    fn from(value: ClientHello<'_>) -> Self {
        CapturedClientHello {
            data: format!("{value:?}"),
        }
    }
}

#[derive(Debug)]
pub struct LoggingResolvesServerCert {
    pub client_hello: Arc<Mutex<Option<CapturedClientHello>>>,
    key: Arc<CertifiedKey>,
}

impl LoggingResolvesServerCert {
    pub fn new(key: CertifiedKey) -> Self {
        Self {
            client_hello: Arc::new(Mutex::new(None)),
            key: Arc::new(key),
        }
    }
}

impl ResolvesServerCert for LoggingResolvesServerCert {
    fn resolve(
        &self,
        client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        if let Ok(mut guard) = self.client_hello.lock() {
            let _ = guard.insert(client_hello.into());
        }
        Some(self.key.clone())
    }
}

#[derive(Debug, Default, Clone)]
pub struct CapturedResolveClientCert {
    pub data: String,
}

impl CapturedResolveClientCert {
    fn new(root_hint_subjects: &[&[u8]], sigschemes: &[SignatureScheme]) -> Self {
        Self {
            data: format!("{root_hint_subjects:?} {sigschemes:?}"),
        }
    }
}

#[derive(Debug)]
pub struct LoggingResolvesClientCert {
    capture: Arc<Mutex<Option<CapturedResolveClientCert>>>,
}

impl Default for LoggingResolvesClientCert {
    fn default() -> Self {
        Self {
            capture: Arc::new(Mutex::new(None)),
        }
    }
}

impl ResolvesClientCert for LoggingResolvesClientCert {
    fn resolve(
        &self,
        root_hint_subjects: &[&[u8]],
        sigschemes: &[SignatureScheme],
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        if let Ok(mut guard) = self.capture.lock() {
            let _ = guard.insert(CapturedResolveClientCert::new(
                root_hint_subjects,
                sigschemes,
            ));
        }
        None
    }

    fn has_certs(&self) -> bool {
        true
    }
}
