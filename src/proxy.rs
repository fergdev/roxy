use chrono::Utc;
use http::Uri;
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use rcgen::KeyUsagePurpose;
use rcgen::{CertificateParams, DnType, IsCa};
use rustls::RootCertStore;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{SignatureScheme, pki_types::*};
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace};

use rustls::pki_types::{PrivateKeyDer, ServerName};
use rustls::server::ServerConfig;
use snowflake::SnowflakeIdGenerator;
use tokio::io::BufReader;
use tokio::{io::AsyncWriteExt, sync::Mutex};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use std::sync::Arc;
use tokio::net::TcpStream;

use crate::certs::RoxyCA;
use crate::flow::{
    CertInfo, Flow, FlowStore, InterceptedRequest, InterceptedResponse, Scheme, read_http_request,
    read_http_response,
};
use crate::interceptor::ScriptEngine;

static ID_GENERATOR: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let generator = SnowflakeIdGenerator::new(1, 1);
    Mutex::new(generator)
});

async fn next_id() -> i64 {
    ID_GENERATOR.lock().await.generate()
}

pub fn start_proxy(
    port: u16,
    roxy_ca: RoxyCA,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> std::io::Result<()> {
    let host = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&host).await.unwrap();
        info!("Proxy listening on {}", host);

        let ca = Arc::new(roxy_ca);
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            info!("Accepted connection from {}", stream.peer_addr().unwrap());

            let io = TokioIo::new(stream);

            let ca2 = ca.clone();
            let script_engine = script_engine.clone();
            let fs = flow_store.clone();
            tokio::task::spawn(async move {
                if let Err(err) = ServerBuilder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .serve_connection(
                        io,
                        service_fn({
                            move |req| proxy(req, fs.clone(), ca2.clone(), script_engine.clone())
                        }),
                    )
                    .with_upgrades()
                    .await
                {
                    error!("Failed to serve connection: {:?}", err);
                }
            });
        }
    });
    Ok(())
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
    fs: FlowStore,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    debug!("req: {:?}", req);

    let id = next_id().await;
    let flow = fs.new_flow(id).await;

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let intercepted = InterceptedRequest {
        timestamp: Utc::now(),
        scheme: match parts.uri.scheme_str() {
            Some("https") => Scheme::Https,
            Some(_) => Scheme::Http,
            None => Scheme::Http,
        },
        host: parts.uri.host().unwrap_or("").to_string(),
        port: parts.uri.port_u16().unwrap_or(80),
        path: parts.uri.path().to_string(),
        method: parts.method.to_string(),
        version: match parts.version {
            hyper::Version::HTTP_10 => 0,
            hyper::Version::HTTP_11 => 1,
            _ => 1,
        },
        headers: parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect(),
        body: Some(String::from_utf8_lossy(&body_bytes).to_string()),
    };

    if Method::CONNECT == parts.method {
        debug!("CONNECT request: {:?}", parts.uri);
        if let Some(addr) = host_addr(&parts.uri) {
            let req =
                Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(flow, upgraded, addr, ca, script_engine).await {
                            error!("server io error: {}", e);
                        };
                    }
                    Err(e) => error!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(empty()))
        } else {
            debug!("CONNECT host is not socket addr: {:?}", parts.uri);
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        let mut guard = flow.write().await;

        guard.request = Some(intercepted);
        script_engine
            .as_ref()
            .map(|engine| engine.intercept_request(&mut guard));

        let host = guard.request.as_ref().unwrap().host.clone();
        let port = guard.request.as_ref().unwrap().port;

        drop(guard);

        let stream = TcpStream::connect((host, port)).await.unwrap();
        let io = TokioIo::new(stream);

        let (mut sender, conn) = ClientBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(io)
            .await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                debug!("Connection failed: {:?}", err);
            }
        });

        let req = Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());

        let resp = sender.send_request(req).await?;

        let (parts, body) = resp.into_parts();
        let body_bytes = body.collect().await?.to_bytes();

        let intercepted = InterceptedResponse {
            reason: parts.status.canonical_reason().unwrap_or("").to_string(),
            status: parts.status.as_u16(),
            timestamp: Utc::now(),
            version: match parts.version {
                hyper::Version::HTTP_10 => 0,
                hyper::Version::HTTP_11 => 1,
                _ => 1,
            },
            headers: parts
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: Some(String::from_utf8_lossy(&body_bytes).to_string()),
        };

        let mut guard = flow.write().await;
        guard.response = Some(intercepted);

        script_engine
            .as_ref()
            .map(|engine| engine.intercept_response(&mut guard));

        drop(guard);

        let resp = Response::from_parts(parts, full(body_bytes.clone()));
        Ok(resp.map(|b| b.boxed()))
    }
}

fn host_addr(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

async fn tunnel(
    flow: Arc<RwLock<Flow>>,
    upgraded: Upgraded,
    target_addr: String,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> std::io::Result<()> {
    // Connect to remote server
    let client_stream = TokioIo::new(upgraded);

    // Ensure the default CryptoProvider is installed before using rustls builders
    debug!("Established tunnel with {}", target_addr);
    let host = target_addr.split(':').next().unwrap();

    let mut params = CertificateParams::new(vec![host.to_string()]).unwrap();
    params.distinguished_name.push(DnType::CommonName, host);
    params.is_ca = IsCa::NoCa;
    params.not_before = rcgen::date_time_ymd(2023, 1, 1); // Set sane date
    params.not_after = rcgen::date_time_ymd(2030, 1, 1);

    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    let leaf = params.self_signed(&ca.key_pair).unwrap();
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::try_from(ca.key_pair.serialize_der()).unwrap(),
        )
        .unwrap();

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let client_tls = acceptor.accept(client_stream).await.unwrap();

    let (mut cr, mut cw) = tokio::io::split(BufReader::new(client_tls));

    let req = read_http_request(&mut cr, host.to_string(), 443, Scheme::Https)
        .await
        .unwrap();

    let mut g_flow = flow.write().await;
    g_flow.request = Some(req);

    debug!("Invoking script_engine");

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut g_flow));

    let req_bytes = g_flow
        .request
        .as_ref()
        .map(|r| r.to_bytes())
        .unwrap_or_else(|| {
            info!("No request to send upstream");
            Vec::new()
        });

    drop(g_flow);

    // let target_host = req_uri.clone();
    let target_host = target_addr.clone();
    info!(
        "DEBUGPRINT[37]: proxy.rs:297: target_host={:#?}",
        target_host
    );
    let uri = target_host.parse::<Uri>().unwrap();

    let target_host = uri.host().unwrap();
    let target_port = uri.port_u16().unwrap_or(443);

    // Connect to upstream server with TLS
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let cert_logger = Arc::new(LoggingCertVerifier::new());

    let client_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(cert_logger.clone())
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host.to_string()).unwrap();
    debug!(
        "Connecting to upstream server: {}:{}",
        target_host, target_port
    );

    let server_stream = tokio::net::TcpStream::connect(target_addr).await.unwrap();
    let upstream_tls = connector.connect(server_name, server_stream).await.unwrap();

    let (mut sr, mut sw) = tokio::io::split(BufReader::new(upstream_tls));

    // Forward request to server
    sw.write_all(&req_bytes).await?;
    sw.flush().await?;

    // Read and log server response
    let resp = read_http_response(&mut sr).await.unwrap();
    let mut g_flow = flow.write().await;
    g_flow.cert_info = Some(cert_logger.certs.lock().unwrap().to_owned());
    g_flow.response = Some(resp);

    if let Some(engine) = &script_engine {
        engine.intercept_response(&mut g_flow).unwrap();
    }

    let bytes = g_flow
        .response
        .as_ref()
        .map(|r| r.to_bytes())
        .unwrap_or_else(|| {
            debug!("No response to send back");
            Vec::new()
        });

    cw.write_all(&bytes).await?;

    drop(g_flow);

    cw.flush().await?;
    Ok(())
}

#[derive(Debug)]
pub struct LoggingCertVerifier {
    certs: std::sync::Mutex<Vec<CertInfo>>,
}

impl LoggingCertVerifier {
    pub fn new() -> Self {
        LoggingCertVerifier {
            certs: std::sync::Mutex::new(vec![]),
        }
    }
}

impl Default for LoggingCertVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCertVerifier for LoggingCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        trace!("Verifying server certificate for: {:?}", server_name);

        for cert in intermediates.iter() {
            self.certs
                .lock()
                .unwrap()
                .push(CertInfo::from_der(cert).unwrap());
        }

        // Always accept the cert (do not use this in production!)
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Skip verification (do not use in production)
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Skip verification (do not use in production)
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
