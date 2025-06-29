use chrono::Utc;
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use rcgen::KeyUsagePurpose;
use rcgen::{CertificateParams, DnType, IsCa};
use rustls::RootCertStore;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{SignatureScheme, pki_types::*};
use tracing::debug;
use x509_parser::prelude::*;

use rustls::pki_types::{PrivateKeyDer, ServerName};
use rustls::server::ServerConfig;
use snowflake::SnowflakeIdGenerator;
use tokio::io::BufReader;
use tokio::sync::mpsc::UnboundedSender;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

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
// use tracing::println;

use crate::certs::RoxyCA;
use crate::event::{AppEvent, Event};
use crate::flow::{InterceptedRequest, InterceptedResponse, read_http_request, read_http_response};
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
    tx: UnboundedSender<Event>,
    roxy_ca: RoxyCA,
    script_engine: Option<ScriptEngine>,
) -> std::io::Result<()> {
    let host = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&host).await.unwrap();
        println!("Proxy listening on {}", host);

        let ca = Arc::new(roxy_ca);
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            println!("Accepted connection from {}", stream.peer_addr().unwrap());

            let io = TokioIo::new(stream);

            let tx = tx.clone();
            let ca2 = ca.clone();
            let script_engine = script_engine.clone();
            tokio::task::spawn(async move {
                if let Err(err) = ServerBuilder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .serve_connection(
                        io,
                        service_fn({
                            move |req| proxy(req, tx.clone(), ca2.clone(), script_engine.clone())
                        }),
                    )
                    .with_upgrades()
                    .await
                {
                    println!("Failed to serve connection: {:?}", err);
                }
            });
        }
    });
    Ok(())
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
    tx: UnboundedSender<Event>,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    println!("req: {:?}", req);
    let id = next_id().await;

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let intercepted = InterceptedRequest {
        id,
        timestamp: Utc::now(),
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
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

    let _ = tx.send(Event::App(crate::event::AppEvent::Request(
        intercepted.clone(),
    )));

    if Method::CONNECT == parts.method {
        println!("CONNECT request: {:?}", parts.uri);
        if let Some(addr) = host_addr(&parts.uri) {
            let req =
                Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr, tx, ca, script_engine).await {
                            eprintln!("server io error: {}", e);
                        };
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(empty()))
        } else {
            eprintln!("CONNECT host is not socket addr: {:?}", parts.uri);
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        println!("HTTP request: {:?}", parts.uri);
        let host = parts.uri.host().expect("uri has no host");
        let port = parts.uri.port_u16().unwrap_or(80);

        let stream = TcpStream::connect((host, port)).await.unwrap();
        let io = TokioIo::new(stream);

        let (mut sender, conn) = ClientBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(io)
            .await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let req = Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());

        let resp = sender.send_request(req).await?;

        let (parts, body) = resp.into_parts();
        let body_bytes = body.collect().await?.to_bytes();

        let intercepted = InterceptedResponse {
            id,
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

        let _ = tx.send(Event::App(crate::event::AppEvent::Response(
            intercepted.clone(),
        )));
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
    upgraded: Upgraded,
    target_host: String,
    tx: UnboundedSender<Event>,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> std::io::Result<()> {
    // Connect to remote server
    let client_stream = TokioIo::new(upgraded);

    // Ensure the default CryptoProvider is installed before using rustls builders
    println!("Established tunnel with {}", target_host);
    let mut split = target_host.split(':');
    let target_host = split.next().unwrap_or("localhost");
    let target_port = split
        .next()
        .unwrap_or("443")
        .to_string()
        .parse::<u16>()
        .unwrap_or(443);
    println!("host {}", target_host);

    let mut params = CertificateParams::new(vec![target_host.to_string()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, target_host);
    params.is_ca = IsCa::NoCa;
    params.not_before = rcgen::date_time_ymd(2023, 1, 1); // Set sane date
    params.not_after = rcgen::date_time_ymd(2030, 1, 1);

    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    // params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let leaf = params.self_signed(&ca.key_pair).unwrap();
    std::fs::write("/tmp/roxy-cert.pem", leaf.pem()).unwrap();
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::try_from(ca.key_pair.serialize_der()).unwrap(),
        )
        .unwrap();

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let client_tls = acceptor.accept(client_stream).await.unwrap();

    // Connect to upstream server with TLS
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let client_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LoggingCertVerifier))
        // .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host.to_string()).unwrap();
    let server_stream = tokio::net::TcpStream::connect((target_host.to_string(), target_port))
        .await
        .unwrap();
    let upstream_tls = connector.connect(server_name, server_stream).await.unwrap();

    // Now parse HTTP requests/responses between client_tls <-> upstream_tls, logging each
    mitm_bidirectional(client_tls, upstream_tls, tx, script_engine)
        .await
        .unwrap();
    Ok(())
}

pub async fn mitm_bidirectional<C, S>(
    client_tls: C,
    server_tls: S,
    tx: UnboundedSender<Event>,
    script_engine: Option<ScriptEngine>,
) -> anyhow::Result<()>
where
    C: AsyncRead + AsyncWrite + Unpin,
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut cr, mut cw) = tokio::io::split(BufReader::new(client_tls));
    let (mut sr, mut sw) = tokio::io::split(BufReader::new(server_tls));

    loop {
        // Read and log client request
        let mut req = match read_http_request(&mut cr, 0).await {
            Ok(r) => r,
            Err(_) => break,
        };
        let _ = tx.send(Event::App(AppEvent::Request(req.clone())));

        println!("Invoking script_engine");
        script_engine
            .as_ref()
            .map(|engine| engine.intercept_request(&mut req));

        // Forward request to server
        sw.write_all(&req.to_bytes()).await?;
        sw.flush().await?;

        // Read and log server response
        let mut resp = match read_http_response(&mut sr, req.id).await {
            Ok(r) => r,
            Err(_) => break,
        };
        debug!("Invoking script_engine");
        script_engine
            .as_ref()
            .map(|engine| engine.intercept_response(&mut resp));
        let _ = tx.send(Event::App(AppEvent::Response(resp.clone())));

        println!("{}", String::from_utf8_lossy(&resp.clone().to_bytes()));
        // Forward response to client
        cw.write_all(&resp.to_bytes()).await?;
        cw.flush().await?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct LoggingCertVerifier;

impl ServerCertVerifier for LoggingCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        println!("🔍 Verifying server certificate for: {:?}", server_name);

        fn print_cert(cert: &CertificateDer<'_>, label: &str) {
            match parse_x509_certificate(cert.as_ref()) {
                Ok((_, parsed)) => {
                    println!("📄 {}:", label);
                    println!("    Subject: {}", parsed.subject());
                    println!("    Issuer : {}", parsed.issuer());
                    println!("    Not Before: {}", parsed.validity().not_before);
                    println!("    Not After : {}", parsed.validity().not_after);
                }
                Err(err) => {
                    println!("❌ Failed to parse {}: {:?}", label, err);
                }
            }
        }

        print_cert(end_entity, "End-entity cert");
        for (i, cert) in intermediates.iter().enumerate() {
            print_cert(cert, &format!("Intermediate cert {}", i + 1));
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
