use std::{error::Error, io, sync::Arc};

use bytes::{Buf, Bytes, BytesMut};
use futures_util::future;
use h3::{client::RequestStream, error::StreamError, ext::Protocol};
use http_body_util::BodyExt;
use quinn::{VarInt, crypto::rustls::QuicClientConfig};

use crate::{
    alpn::alp_h3,
    body::BytesBody,
    http::{HttpEmitter, HttpError, HttpResponse},
    uri::RUri,
};
use http::{
    Method, Request,
    header::{HOST, TE, TRAILER},
};
use rustls::RootCertStore;
use tracing::{debug, error, info, trace};

use h3_quinn::{BidiStream, quinn};

pub async fn h3_with_proxy(
    proxy_uri: Option<&RUri>,
    roots: Arc<RootCertStore>,
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, HttpError> {
    h3_with_proxy_inner(proxy_uri, roots, request, emitter)
        .await
        .map_err(|_| HttpError::ProxyConnect)
}

async fn h3_with_proxy_inner(
    proxy_uri: Option<&RUri>,
    roots: Arc<RootCertStore>,
    request: Request<BytesBody>,
    emitter: &dyn HttpEmitter,
) -> Result<HttpResponse, Box<dyn Error>> {
    debug!("Proxy_addr  {:?}", proxy_uri);
    debug!("Target_addr {}", request.uri());

    let connect_uri = proxy_uri.map(|uri| uri.host_port()).unwrap_or(format!(
        "{}:{}",
        request.uri().host().unwrap_or("localhost"),
        request.uri().port_u16().unwrap_or(443)
    ));

    let host_name = proxy_uri.map(|uri| uri.host()).unwrap_or("localhost");
    let socket_addr = tokio::net::lookup_host(connect_uri).await?;

    let mut tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    tls_config.enable_early_data = true;
    tls_config.alpn_protocols = alp_h3();

    let mut quinn_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse()?)?;
    let client_config = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls_config)?));
    quinn_endpoint.set_default_client_config(client_config);

    let mut connection = None;
    for addr in socket_addr {
        emitter.emit(crate::http::HttpEvent::TcpConnect(addr));
        if let Ok(conn) = quinn_endpoint.connect(addr, host_name)?.await {
            connection = Some(conn);
            break;
        }
    }

    let conn = connection.ok_or(io::Error::other(format!(
        "DNS look up for {host_name} failed"
    )))?;

    let (mut driver, mut send_request) = h3::client::builder()
        .enable_extended_connect(true)
        .enable_datagram(true)
        .send_grease(true)
        .build(h3_quinn::Connection::new(conn))
        .await?;

    let drive = tokio::spawn(async move {
        let res = future::poll_fn(|cx| driver.poll_close(cx)).await;
        error!("Connection close {res}");
    });

    if proxy_uri.is_some() {
        let req = http::Request::builder()
            .method(Method::CONNECT)
            .extension(Protocol::CONNECT_UDP)
            .header(HOST, host_name)
            .body(())?;

        let mut stream = send_request.send_request(req).await?;
        stream.finish().await?;
        let resp = stream.recv_response().await?;
        if resp.into_parts().0.status != 200 {
            return Err(Box::new(HttpError::ProxyConnect));
        }
    }

    debug!("REQUEST ...");
    let (mut parts, mut body) = request.into_parts();
    parts.headers.remove(TE); // TODO: SOMETHING funky here
    // Host needs to be removed for H3 to work
    parts.headers.remove(HOST);

    let req = Request::from_parts(parts, ());
    let mut stream = send_request.send_request(req).await?;

    debug!("REQUEST waiting ...");
    while let Some(Ok(frame)) = body.frame().await {
        if let Some(data) = frame.data_ref() {
            stream.send_data(data.clone()).await?; // TODO: this is bad, no clone here
        } else if let Ok(trailer) = frame.into_trailers() {
            stream.send_trailers(trailer).await?;
        }
    }

    stream.finish().await?;
    let resp = stream.recv_response().await?;
    trace!("REQUEST response: {:?} {}", resp.version(), resp.status());
    trace!("REQUEST headers: {:#?}", resp.headers());

    let mut buf = BytesMut::new();
    while let Some(chunk) = stream.recv_data().await? {
        buf.extend_from_slice(chunk.chunk());
    }

    let body = buf.freeze();
    let (response_parts, _) = resp.into_parts();

    let trailers = if response_parts.headers.contains_key(TRAILER) {
        stream.recv_trailers().await?
    } else {
        None
    };
    trace!("REQUEST trailers {:?}", trailers);
    trace!("REQUEST shutting down");

    drive.abort();

    Ok(HttpResponse {
        parts: response_parts,
        body,
        trailers,
    })
}

pub async fn client_h3_wt(
    proxy_uri: Option<&RUri>,
    target_uri: &RUri,
    roots: Arc<RootCertStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    let connect_uri = proxy_uri.unwrap_or(target_uri);

    let addr = tokio::net::lookup_host(connect_uri.host_port())
        .await?
        .next()
        .ok_or("dns found no addresses")?;

    let mut tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    tls_config.enable_early_data = true;
    tls_config.alpn_protocols = alp_h3();

    let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse()?)?;

    let client_config = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls_config)?));
    client_endpoint.set_default_client_config(client_config);

    let conn = client_endpoint.connect(addr, connect_uri.host())?.await?;

    trace!("QUIC connection established");

    let h3_quinn_conn = h3_quinn::Connection::new(conn.clone());

    let (mut driver, mut send_request) = h3::client::builder()
        .enable_datagram(true)
        .enable_web_transport(true)
        .enable_extended_connect(true)
        .send_grease(true)
        .build(h3_quinn_conn)
        .await?;

    tokio::spawn(async move {
        let e = future::poll_fn(|cx| driver.poll_close(cx)).await;
        error!("Closed {e}");
    });

    trace!("sending request ...");

    let req = match http::Request::builder()
        .method(Method::CONNECT)
        .extension(Protocol::WEB_TRANSPORT)
        .uri(target_uri.inner())
        .body(())
    {
        Ok(req) => req,
        Err(e) => {
            error!("oooops {e}");
            return Err(Box::new(StreamError::RemoteClosing));
        }
    };

    let mut stream: RequestStream<BidiStream<Bytes>, Bytes> =
        send_request.send_request(req).await?;
    stream.finish().await?;

    trace!("receiving response ...");
    let resp = stream.recv_response().await?;
    trace!("response: {:?} {}", resp.version(), resp.status());
    trace!("headers: {:#?}", resp.headers());

    if resp.status() != 200 {
        return Err(Box::new(io::Error::other("Connect refused")));
    }

    let (mut wt_tx, mut wt_rx) = conn.accept_bi().await?;
    let _ = wt_rx.read_to_end(66546).await?;
    trace!("Recv data");
    wt_tx.write(b"hey back").await?;
    wt_tx.finish()?;

    conn.send_datagram_wait(Bytes::from_static(b"heloooooooooo"))
        .await?;
    let data = conn.read_datagram().await?;

    info!("datagram {:?}", data);

    conn.close(VarInt::from_u32(0), &[]);
    Ok(())
}
