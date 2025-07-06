use bytes::Bytes;
use chrono::Utc;
use http::{StatusCode, Uri};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use rustls::RootCertStore;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error};

use hyper::upgrade::Upgraded;
use hyper::{Request, Response};
use rustls::pki_types::ServerName;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;
use tokio_rustls::TlsConnector;

use crate::flow::{Flow, FlowStore, Scheme, read_http_response};
use crate::flow::{FlowKind, InterceptedResponse};
use crate::flow::{HttpFlow, HttpsFlow};
use crate::flow::{InterceptedRequest, read_http_request};
use crate::interceptor::ScriptEngine;
use crate::proxy::cert::LoggingCertVerifier;
use std::sync::Arc;

use super::full;
use super::peekable_duplex::PeekableDuplex;

pub async fn handle_http(
    mut intercepted: InterceptedRequest,
    flow: Arc<RwLock<Flow>>,
    body_bytes: Bytes,
    script_engine: Option<ScriptEngine>,
    fs: FlowStore,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let mut guard = flow.write().await;

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut intercepted));

    let host = intercepted.host.clone();
    let port = intercepted.port;
    let path = intercepted.path.clone();
    let http_flow = HttpFlow::new(intercepted);

    guard.kind = FlowKind::Http(http_flow);

    drop(guard);
    fs.notify();

    let stream = match TcpStream::connect((host.clone(), port)).await {
        Ok(it) => it,
        Err(err) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(full(format!(
                    "Failed to connect to {}: {}: {}",
                    host, port, err
                )))
                .unwrap()
                .map(|b| b.boxed()));
        }
    };
    let io = TokioIo::new(stream);

    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .handshake(io)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            debug!("Connection failed: {:?}", err);
        }
    });

    let req2 = Request::builder()
        .method("GET")
        .uri(path)
        .header("Host", format!("{}:{}", host, port))
        .body(Full::new(body_bytes.clone()).boxed())
        .unwrap();

    let resp = sender.send_request(req2).await?;

    let (parts, body) = resp.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let mut intercepted = InterceptedResponse::from_http(&parts, &body_bytes);

    let mut guard = flow.write().await;

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_response(&mut intercepted));

    match &mut guard.kind {
        FlowKind::Http(http_flow) => {
            http_flow.response = Some(intercepted);
        }
        _ => {
            error!("Flow kind is not Http, cannot set response");
        }
    }

    drop(guard);
    fs.notify();

    let resp = Response::from_parts(parts, full(body_bytes.clone()));
    Ok(resp.map(|b| b.boxed()))
}

pub async fn handle_https(
    peekable: PeekableDuplex<
        tokio_rustls::server::TlsStream<BufReader<PeekableDuplex<TokioIo<Upgraded>>>>,
    >,
    requested_addr: &str,
    handshake: InterceptedRequest,
    flow: Arc<RwLock<Flow>>,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> Result<(), std::io::Error> {
    let host = requested_addr.split(':').next().unwrap_or("localhost");
    let port = requested_addr
        .split(':')
        .nth(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(443);

    let mut reader = BufReader::new(peekable);
    let mut req = read_http_request(&mut reader, host, port, Scheme::Https)
        .await
        .map_err(|e| {
            error!("Failed to read HTTP request: {}", e);
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP request error: {}", e),
            )
        })?;

    let client_tls = reader.into_inner();

    let (mut cr, mut cw) = tokio::io::split(BufReader::new(client_tls));
    let mut g_flow = flow.write().await;

    debug!("Invoking script_engine");

    if let Some(engine) = script_engine.as_ref() {
        engine.intercept_request(&mut req).unwrap()
    }

    let target_addr = req.target_host();
    println!("Target host: {}", requested_addr);
    println!("Target address: {}", target_addr);

    let req_bytes = req.to_bytes();
    let http_flow = HttpsFlow::new(handshake, req);
    g_flow.kind = FlowKind::Https(http_flow);

    drop(g_flow);
    flow_store.notify();

    let uri = target_addr.parse::<Uri>().unwrap();
    let target_host = uri.host().unwrap();

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let cert_logger = Arc::new(LoggingCertVerifier::new());

    let client_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(cert_logger.clone())
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host.to_string()).unwrap();
    debug!("Connecting to upstream server: {}", target_addr);

    let server_stream = tokio::net::TcpStream::connect(target_addr).await.unwrap();
    let upstream_tls = connector.connect(server_name, server_stream).await.unwrap();

    let (mut sr, mut sw) = tokio::io::split(BufReader::new(upstream_tls));

    sw.write_all(&req_bytes).await?;
    sw.flush().await?;

    let mut resp = read_http_response(&mut sr).await.unwrap();

    if let Some(engine) = &script_engine {
        engine.intercept_response(&mut resp).unwrap();
    }

    let bytes = resp.to_bytes();
    let mut g_flow = flow.write().await;
    match &mut g_flow.kind {
        FlowKind::Https(g_flow) => {
            g_flow.cert_info = Some(cert_logger.certs.lock().unwrap().to_owned());
            g_flow.response = Some(resp);
        }
        _ => {
            error!("Flow kind is not Https, cannot set request");
        }
    }

    cw.write_all(&bytes).await?;

    drop(g_flow);
    flow_store.notify();

    cw.flush().await?;

    {
        let mut flow = flow.write().await;
        flow.timing.server_conn_closed = Some(Utc::now());
    }
    Ok(())
}
