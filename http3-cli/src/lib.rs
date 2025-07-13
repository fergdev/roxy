use std::sync::Arc;

use bytes::{Buf, Bytes, BytesMut};
use futures::future;
use http::{Method, Request, Response, Uri, Version, header::HOST};
use http_body_util::Full;
use roxy_shared::init_crypto;
use rustls::pki_types::CertificateDer;
use tracing::{error, info};

use h3_quinn::quinn;

pub static ALPN: &[u8] = b"h3";

pub async fn h3_with_proxy(
    proxy_uri: Uri,
    host_addr: Uri,
    ca_der: CertificateDer<'static>,
    request: Request<()>,
) -> anyhow::Result<Response<Full<Bytes>>> {
    info!("Proxy_addr {}", proxy_uri);
    info!("Host addr  {}", host_addr);

    if proxy_uri.scheme() != Some(&http::uri::Scheme::HTTPS) {
        Err(anyhow::anyhow!("uri scheme must be 'https'"))?
    }

    let auth = proxy_uri.authority().ok_or("uri must have a host").unwrap();
    let port = auth.port_u16().unwrap_or(443);
    let addr = tokio::net::lookup_host((auth.host(), port))
        .await?
        .next()
        .ok_or("dns found no addresses")
        .unwrap();

    info!("DNS lookup for {:?}: {:?}", proxy_uri, addr);

    init_crypto();
    let mut roots = rustls::RootCertStore::empty();

    info!("Loading native certs");
    let cert_result = rustls_native_certs::load_native_certs();

    for err in cert_result.errors.iter() {
        error!("Load cert error {err}");
    }
    if !cert_result.errors.is_empty() {
        panic!("Cert load errors");
    }

    for cert in cert_result.certs {
        if let Err(e) = roots.add(cert) {
            error!("failed to parse trust anchor: {}", e);
        }
    }

    roots.add(ca_der).unwrap();

    info!("Tls config");
    let mut tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    tls_config.enable_early_data = true;
    tls_config.alpn_protocols = vec![ALPN.into()];

    let mut client_endpoint = h3_quinn::quinn::Endpoint::client("[::]:0".parse().unwrap())?;

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));
    client_endpoint.set_default_client_config(client_config);

    info!("quinn connect {addr} {}", auth.host());
    let conn = client_endpoint.connect(addr, auth.host())?.await?;

    info!("QUIC connection established");

    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await?;

    let drive = tokio::spawn(async move {
        let res = future::poll_fn(|cx| driver.poll_close(cx)).await;
        error!("Connection close {res}");
    });

    info!("CONNECT ...");

    let req = http::Request::builder()
        .method(Method::CONNECT)
        .header(HOST, host_addr.authority().unwrap().to_string())
        .body(())
        .unwrap();
    let mut stream = send_request.send_request(req).await?;

    stream.finish().await?;

    info!("CONNECT response ...");

    let resp = stream.recv_response().await?;

    info!("CONNECT response: {:?} {}", resp.version(), resp.status());
    info!("CONNECT headers: {:#?}", resp.headers());

    let mut buf = BytesMut::new();
    while let Some(chunk) = stream.recv_data().await? {
        buf.extend_from_slice(chunk.chunk());
    }

    let buf = buf.freeze();
    info!("CONNECT Body {}", String::from_utf8_lossy(&buf));

    info!("REQUEST ...");
    let mut stream = send_request.send_request(request).await?;

    info!("REQUEST awaiting finish");
    stream.finish().await?;

    info!("REQUEST receiving response ...");

    let resp = stream.recv_response().await?;

    info!("REQUEST response: {:?} {}", resp.version(), resp.status());
    info!("REQUEST headers: {:#?}", resp.headers());

    let mut buf = BytesMut::new();
    while let Some(chunk) = stream.recv_data().await? {
        buf.extend_from_slice(chunk.chunk());
    }
    let buf = buf.freeze();
    info!("REQUEST Body {}", String::from_utf8_lossy(&buf));

    info!("REQUEST shutting down");

    // client_endpoint.wait_idle().await; //TODO: figure out why this is not finishing
    drive.abort();

    let resp_builder = http::Response::builder()
        .status(resp.status())
        .version(Version::HTTP_3); // TODO: match intercepted
    let resp = resp_builder
        .body(Full::new(buf))
        .map_err(|e| anyhow::anyhow!("Failed to make response {e}"))?;
    Ok(resp)
}
