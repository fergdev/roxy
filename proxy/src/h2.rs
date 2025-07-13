use anyhow::anyhow;
use bytes::Bytes;
use http::StatusCode;
use http::Uri;
use http::Version;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper::Response;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::info;
use tracing::warn;
use tracing::{debug, error};

use super::cert::LoggingCertVerifier;
use crate::flow::FlowConnection;
use crate::flow::InterceptedRequest;
use crate::flow::{FlowKind, Http2Flow, InterceptedResponse};
use crate::h1::upstream_https_req;
use crate::interceptor::ScriptEngine;
use crate::proxy::AlpnProtocol;
use crate::utils::IOTypeNotSend;

pub async fn handle_h2<S>(
    socket_addr: SocketAddr,
    client_stream: S,
    target_addr: &str,
    flow_store: crate::flow::FlowStore,
    script_engine: Option<ScriptEngine>,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let executor = hyper_util::rt::tokio::TokioExecutor::new();
    debug!("Spawning H2 client connection handler");
    let client_io = TokioIo::new(client_stream);
    hyper::server::conn::http2::Builder::new(executor)
        .serve_connection(
            client_io,
            service_fn({
                let target_addr = target_addr.to_string();
                move |req| {
                    let target_addr = target_addr.clone();
                    proxy(
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
        .map_err(|e| io::Error::other(format!("Client H2 serve failed: {e}")))
}

async fn proxy(
    socket_addr: SocketAddr,
    target_addr: String,
    req: Request<Incoming>,
    flow_store: crate::flow::FlowStore,
    script_engine: Option<ScriptEngine>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    let (parts, body) = req.into_parts();
    let body = body.collect().await?;

    let flow = flow_store
        .new_flow(FlowConnection { addr: socket_addr })
        .await;

    let uri: Uri = target_addr.parse().unwrap();
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16().unwrap_or(443);
    let body_bytes = body.to_bytes();
    let mut intercepted =
        InterceptedRequest::from_http(crate::flow::Scheme::Https, &parts, host, port, &body_bytes);

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut intercepted));

    let host = intercepted.host.clone();
    let port = intercepted.port;
    let path = intercepted.path.clone();

    let clone = intercepted.clone();
    let mut flow_guard = flow.write().await;
    flow_guard.kind = FlowKind::Http2(Http2Flow::new(intercepted));
    flow_guard.timing.client_conn_established = Some(chrono::Utc::now());
    drop(flow_guard);
    flow_store.notify();

    let server_request = Request::builder()
        .method(parts.method)
        .uri(path)
        .header("Host", format!("{host}:{port}"))
        .body(Full::new(body_bytes.clone()))
        .unwrap();

    let cert_logger = Arc::new(LoggingCertVerifier::new());
    debug!("Handling H2 connection");

    // TODO: offer option to verify downsttream certs or not
    let mut tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(cert_logger.clone())
        .with_no_client_auth();

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let connector = TlsConnector::from(Arc::new(tls_config));

    debug!("Connecting to upstream: {}:{}", host, port);
    let tcp = match TcpStream::connect((host.clone(), port)).await {
        Ok(it) => it,
        Err(err) => {
            warn!("TcpConnect err: {}", err);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from_static(b"Bad Gateway")))
                .unwrap());
        }
    };
    let tls = connector
        .connect(ServerName::try_from(host.clone()).unwrap(), tcp)
        .await
        .unwrap();

    let alpn_bytes = tls.get_ref().1.alpn_protocol();
    let alpn = AlpnProtocol::from_bytes(alpn_bytes);
    info!("alp {:?}", alpn);

    if alpn != AlpnProtocol::Http2 {
        info!("Downgrading to http1.1 h2 not agreed on with server");
        let resp = upstream_https_req(&clone).await;

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
                    FlowKind::Http2(flow) => flow.response = Some(intercepted), // HACK: yeah well
                    // it was downgraded
                    _ => {
                        panic!("Expected Http1Flow");
                    }
                }
                drop(flow_guard);
                flow_store.notify();

                return resp_builder
                    .body(Full::new(body_bytes))
                    .map_err(|e| anyhow!("Error making response {}", e));
            }
            Err(_) => {
                return Ok(Response::builder()
                    .status(502)
                    .body(Full::new(Bytes::from_static(b"Bad Gateway")))
                    .unwrap());
            } // TODO: no unwrap
        }
    }

    debug!("TLS connection established to upstream");
    let stream = IOTypeNotSend::new(TokioIo::new(tls));

    let executor = hyper_util::rt::tokio::TokioExecutor::new();
    let (mut upstream_sender, upstream_conn) =
        match hyper::client::conn::http2::handshake(executor.clone(), stream).await {
            Ok(res) => res,
            Err(e) => {
                return Err(anyhow!("Error with http handshake {}", e));
            }
        };

    // let executor = hyper_util::rt::tokio::TokioExecutor::new();
    // let (mut upstream_sender, upstream_conn) = timeout(
    //     Duration::from_secs(60), // TODO: add timeout here
    //     hyper::client::conn::http2::handshake(executor.clone(), stream),
    // )
    // .await
    // .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "H2 upstream handshake timed out"))?
    // .map_err(|e| io::Error::other(format!("H2 upstream handshake failed: {e}")))?;

    debug!("H2 upstream connection established");
    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            error!("Upstream H2 connection error: {}", e);
        }
    });

    match upstream_sender.send_request(server_request).await {
        Ok(res) => {
            let (parts, body) = res.into_parts();
            let body = body.collect().await?;
            let body_bytes = body.to_bytes();

            let mut intercepted = InterceptedResponse::from_http(&parts, &body_bytes);
            script_engine
                .as_ref()
                .map(|engine| engine.intercept_response(&mut intercepted));

            let body_bytes = intercepted.body.clone();
            let mut resp_builder = http::Response::builder()
                .status(intercepted.status)
                .version(Version::HTTP_2); // TODO: match intercepted

            for (k, v) in &intercepted.mapped_headers() {
                resp_builder = resp_builder.header(k, v);
            }

            let mut flow_guard = flow.write().await;
            match &mut flow_guard.kind {
                FlowKind::Http2(flow) => flow.response = Some(intercepted),
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
            .unwrap()),
    }
}
