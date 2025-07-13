use anyhow::anyhow;
use bytes::Bytes;
use http::header::HOST;
use http::response::Parts;
use http::{HeaderValue, Uri, Version};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http1::SendRequest;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use hyper::{Request, Response};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;
use tokio_rustls::TlsConnector;

use crate::cert::LoggingCertVerifier;
use crate::flow::InterceptedRequest;
use crate::flow::{FlowConnection, FlowStore};
use crate::flow::{FlowKind, InterceptedResponse};
use crate::flow::{HttpFlow, HttpsFlow};
use crate::interceptor::ScriptEngine;
use crate::utils::IOTypeNotSend;
use std::io::{self, Error};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub async fn handle_http(
    socket_addr: SocketAddr,
    client_request: Request<hyper::body::Incoming>,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    let (parts, body) = client_request.into_parts();
    let body = body.collect().await?;

    let body_bytes = body.to_bytes();

    let flow = flow_store
        .new_flow(FlowConnection { addr: socket_addr })
        .await;

    let host = parts.uri.host().unwrap_or("localhost");
    let port = parts.uri.port_u16().unwrap_or(80);

    let mut intercepted =
        InterceptedRequest::from_http(crate::flow::Scheme::Http, &parts, host, port, &body_bytes);

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut intercepted));

    let clone = intercepted.clone();
    let http_flow = HttpFlow::new(intercepted);

    let mut flow_guard = flow.write().await;
    flow_guard.kind = FlowKind::Http(http_flow);

    drop(flow_guard);
    flow_store.notify();

    info!("Matching scheme");
    let resp = match clone.scheme {
        crate::flow::Scheme::Http => upstream_http_req(&clone).await,
        crate::flow::Scheme::Https => upstream_https_req(&clone).await,
    };

    match resp {
        Ok((parts, body_bytes)) => {
            let mut intercepted = InterceptedResponse::from_http(&parts, &body_bytes);
            script_engine
                .as_ref()
                .map(|engine| engine.intercept_response(&mut intercepted));

            let body_bytes = intercepted.body.clone();
            let mut resp_builder = http::Response::builder()
                .status(intercepted.status)
                .version(Version::HTTP_11); // TODO: match intercepted

            for (k, v) in &intercepted.mapped_headers() {
                resp_builder = resp_builder.header(k, v);
            }

            let mut flow_guard = flow.write().await;
            match &mut flow_guard.kind {
                FlowKind::Http(flow) => flow.response = Some(intercepted),
                _ => {
                    panic!("Expected Http1Flow");
                }
            }
            drop(flow_guard);
            flow_store.notify();

            resp_builder
                .body(Full::new(body_bytes))
                .map_err(|e| anyhow!("Error making response {}", e))
        }
        Err(_) => Ok(Response::builder()
            .status(502)
            .body(Full::new(Bytes::from_static(b"Bad Gateway")))
            .unwrap()), // TODO: no unwrap
    }
}

pub async fn handle_https<S>(
    socket_addr: SocketAddr,
    client_stream: S,
    target_addr: &str,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> Result<(), std::io::Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    debug!("Spawning H1 client connection handler");
    let client_io = TokioIo::new(client_stream);

    ServerBuilder::new()
        .preserve_header_case(true)
        .keep_alive(true)
        .serve_connection(
            client_io,
            service_fn({
                move |req| {
                    proxy_https(
                        socket_addr,
                        target_addr,
                        req,
                        flow_store.clone(),
                        script_engine.clone(),
                    )
                }
            }),
        )
        .await
        .map_err(|e| std::io::Error::other(format!("Invalid server name: {target_addr} err: {e}")))
}

async fn proxy_https(
    socket_addr: SocketAddr,
    target_addr: &str,
    req: Request<Incoming>,
    flow_store: crate::flow::FlowStore,
    script_engine: Option<ScriptEngine>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    let (parts, body) = req.into_parts();
    let body = body.collect().await?;

    let body_bytes = body.to_bytes();

    let flow = flow_store
        .new_flow(FlowConnection { addr: socket_addr })
        .await;
    let uri: Uri = target_addr.parse().unwrap();
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16().unwrap_or(443);
    info!("target {}:{}", host, port);
    let mut intercepted =
        InterceptedRequest::from_http(crate::flow::Scheme::Https, &parts, host, port, &body_bytes);

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut intercepted));

    let clone = intercepted.clone(); // HACK: do not clone here

    let mut flow_guard = flow.write().await;
    flow_guard.kind = FlowKind::Https(HttpsFlow::new(intercepted));
    flow_guard.timing.client_conn_established = Some(chrono::Utc::now());
    drop(flow_guard);
    flow_store.notify();

    info!("Matching scheme");
    let resp = match clone.scheme {
        crate::flow::Scheme::Http => upstream_http_req(&clone).await,
        crate::flow::Scheme::Https => upstream_https_req(&clone).await,
    };

    match resp {
        Ok((parts, body_bytes)) => {
            let mut intercepted = InterceptedResponse::from_http(&parts, &body_bytes);
            script_engine
                .as_ref()
                .map(|engine| engine.intercept_response(&mut intercepted));

            let body_bytes = intercepted.body.clone();
            let mut resp_builder = http::Response::builder()
                .status(intercepted.status)
                .version(Version::HTTP_11); // TODO: match intercepted

            for (k, v) in &intercepted.mapped_headers() {
                resp_builder = resp_builder.header(k, v);
            }

            let mut flow_guard = flow.write().await;
            match &mut flow_guard.kind {
                FlowKind::Https(flow) => flow.response = Some(intercepted),
                _ => {
                    panic!("Expected Http1Flow");
                }
            }
            drop(flow_guard);
            flow_store.notify();

            resp_builder
                .body(Full::new(body_bytes))
                .map_err(|e| anyhow!("Error making response {}", e))
        }
        Err(_) => Ok(Response::builder()
            .status(502)
            .body(Full::new(Bytes::from_static(b"Bad Gateway")))
            .unwrap()), // TODO: no unwrap
    }
}

// TODO: cache connections
pub async fn get_upstream(target_addr: &str) -> anyhow::Result<SendRequest<Full<Bytes>>> {
    let cert_logger = Arc::new(LoggingCertVerifier::new());
    debug!("Handling H1 connection");

    // TODO: offer option to verify downsttream certs or not
    let mut tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(cert_logger.clone())
        .with_no_client_auth();

    debug!("Using target address: {}", target_addr);
    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let tls_connector = TlsConnector::from(Arc::new(tls_config));

    debug!("Connecting to upstream: {}", target_addr);
    let uri: Uri = target_addr.parse().unwrap();
    let host = uri.host().unwrap_or("localhost").to_owned();
    let tcp = TcpStream::connect(target_addr).await?;
    // TODO: map to tls error
    let tls = tls_connector
        .connect(ServerName::try_from(host.clone()).unwrap(), tcp)
        .await
        .map_err(|e| std::io::Error::other(format!("H1 TLS handshake failed: {e}")))?;

    debug!("TLS connection established to upstream");
    let stream = IOTypeNotSend::new(TokioIo::new(tls));

    let (upstream_sender, upstream_conn) = timeout(
        Duration::from_secs(60),
        hyper::client::conn::http1::handshake(stream),
    )
    .await
    // TODO: map to tls error
    .map_err(|_| Error::new(io::ErrorKind::TimedOut, "H1 upstream handshake timed out"))?
    .map_err(|e| std::io::Error::other(format!("H1 upstream handshake failed: {e}")))?;

    debug!("H1 upstream connection established");
    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            error!("Upstream H1 connection error: {}", e);
        }
    });

    Ok(upstream_sender)
}

pub async fn upstream_http_req(req: &InterceptedRequest) -> Result<(Parts, Bytes), anyhow::Error> {
    info!("Upstream http");
    let host = req.host.clone();
    let port = req.port;
    info!("Connecting to {}:{}", host, port);
    let stream = TcpStream::connect((host.clone(), port)).await?;
    let io = TokioIo::new(stream);

    info!("ClientBuilder");
    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .handshake(io)
        .await?;

    info!("Spawn");
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            warn!("Connection failed: {:?}", err);
        }
    });

    let server_request = Request::builder()
        .method(req.method.as_str())
        .uri(req.uri().unwrap()) // TODO: don't unwrap
        .header("Host", format!("{host}:{port}"))
        .body(Full::new(req.body.clone()).boxed())
        .unwrap();

    info!("Send request");
    let res = sender.send_request(server_request).await?;
    let (parts, body) = res.into_parts();
    let body = body.collect().await?;
    let body_bytes = body.to_bytes();
    Ok((parts, body_bytes))
}

pub async fn upstream_https_req(req: &InterceptedRequest) -> Result<(Parts, Bytes), anyhow::Error> {
    info!("Upstream https");
    let host = req.host.clone();
    let port = req.port;

    let mut new_req = Request::builder()
        .method(req.method.as_str())
        .uri(req.path.clone())
        .header("Host", format!("{host}:{port}"))
        .body(Full::new(req.body.clone()))
        .unwrap();

    let hv = HeaderValue::from_str(&host).unwrap(); // TODO: no unwrap
    new_req.headers_mut().insert(HOST, hv);

    let mut sender = get_upstream(&format!("{host}:{port}")).await?;

    let res = sender.send_request(new_req).await?;
    let (parts, body) = res.into_parts();
    let body = body.collect().await?;
    let body_bytes = body.to_bytes();
    Ok((parts, body_bytes))
}
