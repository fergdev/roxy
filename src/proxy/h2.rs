use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper::Response;
use hyper::body::Incoming;
use hyper::client::conn::http2::SendRequest;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tracing::{debug, error};

use crate::flow::Flow;
use crate::flow::{FlowKind, Http2Flow, InterceptedRequest, InterceptedResponse, Scheme};
use crate::{notify_error, notify_info};

use super::cert::LoggingCertVerifier;

pub async fn handle_h2<S>(
    client_stream: S,
    target_addr: &str,
    flow: Arc<RwLock<Flow>>,
    connect: InterceptedRequest,
    flow_store: crate::flow::FlowStore,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    flow.write().await.kind = FlowKind::Http2(Http2Flow::new(connect));
    flow_store.notify();
    let cert_logger = Arc::new(LoggingCertVerifier::new());
    debug!("Handling H2 connection");

    // TODO: offer option to verify downsttream certs or not
    let mut tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(cert_logger.clone())
        .with_no_client_auth();

    debug!("Using target address: {}", target_addr);
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let connector = TlsConnector::from(Arc::new(tls_config));

    debug!("Connecting to upstream: {}", target_addr);
    let tcp = TcpStream::connect(target_addr).await?;
    let tls = connector
        .connect(ServerName::try_from("localhost").unwrap(), tcp)
        .await
        .unwrap();

    debug!("TLS connection established to upstream");
    let stream = IOTypeNotSend::new(TokioIo::new(tls));

    let executor = hyper_util::rt::tokio::TokioExecutor::new();
    let (upstream_sender, upstream_conn) = timeout(
        Duration::from_secs(5),
        hyper::client::conn::http2::handshake(executor.clone(), stream),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "H2 upstream handshake timed out"))? // outer Result
    .map_err(|e| {
        notify_info!("H2 upstream handshake failed: {e}");
        io::Error::new(
            io::ErrorKind::Other,
            format!("H2 upstream handshake failed: {e}"),
        )
    })?;

    debug!("H2 upstream connection established");
    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            error!("Upstream H2 connection error: {}", e);
            notify_error!("Upstream H2 connection error: {}", e);
        }
    });
    let mut flow_guard = flow.write().await;
    match &mut flow_guard.kind {
        FlowKind::Http2(flow) => {
            let certs = cert_logger.certs.lock().unwrap().to_owned();
            notify_info!("H2 with {} certs", certs.len());
            flow.cert_info = Some(certs);
        }
        _ => {
            panic!("Expected Http2Flow");
        }
    }
    drop(flow_guard);
    flow_store.notify();

    debug!("Spawning H2 client connection handler");
    let client_io = TokioIo::new(client_stream);
    hyper::server::conn::http2::Builder::new(executor)
        .serve_connection(
            client_io,
            service_fn({
                move |req| {
                    proxy(
                        req,
                        upstream_sender.clone(),
                        flow.clone(),
                        flow_store.clone(),
                    )
                }
            }),
        )
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Client H2 serve failed: {e}")))
}

async fn proxy(
    req: Request<Incoming>,
    mut sender: SendRequest<Full<Bytes>>,
    flow: Arc<RwLock<Flow>>,
    flow_store: crate::flow::FlowStore,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let (parts, body) = req.into_parts();
    let body = body.collect().await?;

    notify_info!("⇨ H2 request: {:?}", parts.version);

    let body_bytes = body.to_bytes();
    let mut flow_guard = flow.write().await;
    match &mut flow_guard.kind {
        FlowKind::Http2(flow) => {
            let intercepted = InterceptedRequest::from_http(&parts, &body_bytes);
            flow.requests.push(intercepted);
        }
        _ => {
            panic!("Expected Http2Flow");
        }
    }
    flow_guard.timing.client_conn_established = Some(chrono::Utc::now());
    drop(flow_guard);
    flow_store.notify();

    let new_req = http::Request::from_parts(parts, Full::new(body_bytes));

    match sender.send_request(new_req).await {
        Ok(res) => {
            notify_info!(
                "⇨ H2 upstream response: {} {:?}",
                res.status(),
                res.version()
            );

            let (parts, body) = res.into_parts();
            let body = body.collect().await?;
            let body_bytes = body.to_bytes();

            let intercepted = InterceptedResponse::from_http(&parts, &body_bytes);

            let mut flow_guard = flow.write().await;
            match &mut flow_guard.kind {
                FlowKind::Http2(flow) => {
                    flow.responses.push(intercepted);
                }
                _ => {
                    panic!("Expected Http2Flow");
                }
            }
            drop(flow_guard);
            flow_store.notify();

            let resp = http::Response::from_parts(parts, Full::new(body_bytes));
            Ok(resp)
        }
        Err(e) => {
            notify_info!("⇨ H2 upstream send error: {e}");
            Ok(Response::builder()
                .status(502)
                .body(Full::new(Bytes::from_static(b"Bad Gateway")))
                .unwrap())
        }
    }
}

struct IOTypeNotSend<S> {
    stream: TokioIo<S>,
}

impl<S> IOTypeNotSend<S> {
    fn new(stream: TokioIo<S>) -> Self {
        Self { stream }
    }
}

impl<S: AsyncWrite + Unpin> hyper::rt::Write for IOTypeNotSend<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

impl<S: AsyncRead + Unpin> hyper::rt::Read for IOTypeNotSend<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
