use bytes::Bytes;
use http::HeaderMap;
use http::Request;
use http::Response;
use http::Uri;
use http::uri::InvalidUri;
use http::{Method, header::HOST, response::Parts};
use http_body_util::BodyExt;
use http_body_util::Empty;
use hyper::client::conn::http1;
use hyper::rt::Read;
use hyper::rt::Write;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::tokio::WithHyperIo;
use rustls::pki_types::InvalidDnsNameError;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use tracing::warn;

use tokio::net::TcpStream;
use tracing::error;

use crate::body::BytesBody;
use crate::cert::ClientTlsConnectionData;
use crate::cert::ClientVerificationCapture;
use crate::cert::ServerTlsConnectionData;
use crate::cert::ServerVerificationCapture;
use crate::uri::RUri;
use crate::util::report;
type H1ClientBuilder = hyper::client::conn::http1::Builder;

#[derive(Debug)]
pub struct HttpResponse {
    pub parts: Parts,
    pub body: bytes::Bytes,
    pub trailers: Option<HeaderMap>,
}

pub async fn try_from(res: Response<hyper::body::Incoming>) -> Result<HttpResponse, HttpError> {
    let (parts, body) = res.into_parts();
    let collected = body.collect().await?;
    let trailers = collected.trailers().cloned();
    let body = collected.to_bytes();
    Ok(HttpResponse {
        parts,
        body,
        trailers,
    })
}

#[derive(Debug)]
pub enum HttpEvent {
    TcpConnect(SocketAddr),

    ClientHttpHandshakeStart,
    ClientHttpHandshakeComplete,

    ClientTlsHandshake,
    ClientTlsConn(ClientTlsConnectionData, ServerVerificationCapture),

    ServerTlsConnInitiated,
    ServerTlsConn(ServerTlsConnectionData, ClientVerificationCapture),
    // pub server_conn_initiated: Option<DateTime<Utc>>,
    // pub server_conn_tcp_handshake: Option<DateTime<Utc>>,
    //
    // pub server_conn_tls_initiated: Option<DateTime<Utc>>,
    // pub server_conn_tls_handshake: Option<DateTime<Utc>>,
    //
    // pub server_conn_http_handshake: Option<DateTime<Utc>>,
    //
    // pub first_request_bytes: Option<DateTime<Utc>>,
    // pub request_complete: Option<DateTime<Utc>>,
    //
    // pub first_response_bytes: Option<DateTime<Utc>>,
    // pub response_complete: Option<DateTime<Utc>>,
    //
    // pub client_conn_closed: Option<DateTime<Utc>>,
    // pub server_conn_closed: Option<DateTime<Utc>>,
}

pub trait HttpEmitter: Send + Sync + 'static + std::fmt::Debug {
    fn emit(&self, event: HttpEvent);
}

#[derive(Default, Debug)]
pub struct NoOpListener {}

impl HttpEmitter for NoOpListener {
    fn emit(&self, _event: HttpEvent) {
        // Nothing
    }
}

#[derive(Debug)]
pub enum HttpError {
    Io(std::io::Error),
    Alpn,
    Hyper(hyper::Error),
    HyperUpgrade,
    Http(http::Error),
    Uri,
    InvalidDnsName,
    Timeout,
    ProxyConnect,
    TlsError(std::io::Error),
    BadHost,
}

impl Error for HttpError {}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<InvalidDnsNameError> for HttpError {
    fn from(_: InvalidDnsNameError) -> Self {
        HttpError::InvalidDnsName
    }
}

impl From<InvalidUri> for HttpError {
    fn from(_: InvalidUri) -> Self {
        HttpError::Uri
    }
}

impl From<Elapsed> for HttpError {
    fn from(_: Elapsed) -> Self {
        HttpError::Timeout
    }
}

impl From<std::io::Error> for HttpError {
    fn from(value: std::io::Error) -> Self {
        HttpError::Io(value)
    }
}

impl From<hyper::Error> for HttpError {
    fn from(value: hyper::Error) -> Self {
        HttpError::Hyper(value)
    }
}
impl From<http::Error> for HttpError {
    fn from(value: http::Error) -> Self {
        HttpError::Http(value)
    }
}

pub async fn connect_proxy(
    proxy_uri: &RUri,
    host_uri: &Uri,
) -> Result<WithHyperIo<TcpStream>, HttpError> {
    let addr = proxy_uri.host_port();
    let io = WithHyperIo::new(TcpStream::connect(addr).await?);
    let (mut sender, conn) = H1ClientBuilder::new()
        .title_case_headers(true)
        .handshake(io)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.with_upgrades().await {
            error!("Connection failed: {:?}", err);
        }
    });

    let host = host_uri.host().unwrap_or("localhost");
    let connect_uri = format!("{}:{}", host, host_uri.port_u16().unwrap_or(80));
    let req = http::Request::builder()
        .method(Method::CONNECT)
        .uri(connect_uri.as_str())
        .header(HOST, host)
        .body(Empty::<Bytes>::new())?;

    let resp = sender.send_request(req).await?;
    if resp.status() != 200 {
        return Err(HttpError::ProxyConnect);
    }
    let a = hyper::upgrade::on(resp).await?; // TODO: conversion error
    let parts: hyper::upgrade::Parts<WithHyperIo<TcpStream>> =
        a.downcast().map_err(|_| HttpError::HyperUpgrade)?; // TODO: destroy the stream
    Ok(parts.io)
}

pub async fn uptstream_http_connected(
    request: Request<BytesBody>,
    stream: WithHyperIo<TcpStream>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError> {
    emitter.emit(HttpEvent::ClientHttpHandshakeStart);
    let (mut sender, conn) = H1ClientBuilder::new()
        .title_case_headers(true)
        .handshake(stream)
        .await?;
    emitter.emit(HttpEvent::ClientHttpHandshakeComplete);

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            warn!("Connection failed: {:?}", err);
        }
    });

    try_from(sender.send_request(request).await?).await
}

pub async fn uptstream_http(
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError> {
    let connect_host = format!(
        "{}:{:?}",
        request.uri().host().unwrap_or("localhost"),
        request.uri().port_u16().unwrap_or(80)
    );
    let stream = TcpStream::connect(connect_host).await?;
    let io = WithHyperIo::new(stream);
    uptstream_http_connected(request, io, emitter).await
}

pub async fn uptstream_http_with_proxy(
    proxy_uri: &RUri,
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError> {
    let io = WithHyperIo::new(TcpStream::connect(proxy_uri.host_port()).await?);
    uptstream_http_connected(request, io, emitter).await
}

pub async fn upstream_https<S>(
    tls: S,
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError>
where
    S: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let mut builder = http1::Builder::new();
    builder.title_case_headers(true);

    emitter.emit(HttpEvent::ClientHttpHandshakeStart);
    let (mut sender, upstream_conn) =
        timeout(Duration::from_secs(60), builder.handshake(tls)).await??;

    emitter.emit(HttpEvent::ClientHttpHandshakeComplete);

    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            report(&e);
            error!("Upstream HS connection error: {}", e);
        }
    });
    try_from(sender.send_request(request).await?).await
}

pub async fn upstream_h2<S>(
    tls: S,
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError>
where
    S: Read + Write + Unpin + Send + 'static,
{
    emitter.emit(HttpEvent::ClientHttpHandshakeStart);
    let (mut upstream_sender, upstream_conn) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), tls).await?;

    emitter.emit(HttpEvent::ClientHttpHandshakeComplete);
    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            error!("{e}");
        }
    });

    try_from(upstream_sender.send_request(request).await?).await
}
