use http::Uri;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper_util::rt::TokioIo;
use roxy_shared::RoxyCA;
use tracing::{debug, error, info};

use rustls::pki_types::PrivateKeyDer;
use rustls::server::ServerConfig;

type ServerBuilder = hyper::server::conn::http1::Builder;
use bytes::Bytes;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

use crate::flow::FlowStore;
use crate::h1::{handle_http, handle_https};
use crate::h2::handle_h2;
use crate::h3::start_h3;
use crate::interceptor::ScriptEngine;
use crate::peekable_duplex::PeekableDuplex;
use crate::utils::host_addr;
use crate::ws::{handle_ws, handle_wss};

pub fn start_proxy(
    port: u16,
    roxy_ca: RoxyCA,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> std::io::Result<()> {
    let host = format!("127.0.0.1:{port}");
    let ca = Arc::new(roxy_ca);
    start_h3(port, ca.clone(), script_engine.clone(), flow_store.clone())?;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&host).await.unwrap();
        info!("Proxy listening on {}", host);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            info!("Accepted connection from {}", addr);

            let ca2 = ca.clone();
            let script_engine = script_engine.clone();
            let fs = flow_store.clone();
            tokio::task::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(err) = ServerBuilder::new()
                    .preserve_header_case(true)
                    .serve_connection(
                        io,
                        service_fn({
                            move |req| {
                                proxy(addr, req, fs.clone(), ca2.clone(), script_engine.clone())
                            }
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
    socket_addr: SocketAddr,
    req: Request<hyper::body::Incoming>,
    fs: FlowStore,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    debug!("req: {:?}", req);

    if Method::CONNECT == req.method() {
        let (parts, body) = req.into_parts();
        let body_bytes = body.collect().await?.to_bytes();
        debug!("CONNECT request: {:?}", parts.uri);
        if let Some(addr) = host_addr(&parts.uri) {
            let req =
                Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) =
                            tunnel(socket_addr, upgraded, &addr, ca, script_engine, fs.clone())
                                .await
                        {
                            error!("server io error: {}", e);
                        };
                    }
                    Err(e) => {
                        error!("upgrade error: {}", e);
                    }
                }
            });

            Ok(Response::new(Full::<Bytes>::new(Bytes::from_static(b""))))
        } else {
            debug!("CONNECT host is not socket addr: {:?}", parts.uri);

            Ok(Response::builder()
                .status(502)
                .body(Full::new(Bytes::from_static(
                    b"CONNECT must be to a socket address",
                )))
                .unwrap())
        }
    } else {
        handle_http(socket_addr, req, script_engine, fs).await
    }
}

async fn tunnel(
    socket_addr: SocketAddr,
    upgraded: Upgraded,
    requested_addr: &str,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> io::Result<()> {
    let client_stream = TokioIo::new(upgraded);

    // TODO: Peek less here
    let (stream, peeked_bytes) = PeekableDuplex::new(client_stream, 1024).await?;

    if peeked_bytes.starts_with(b"GET ") {
        return handle_ws(socket_addr, stream, requested_addr, flow_store).await;
    } else if peeked_bytes.starts_with(&[0x16]) {
        info!("Looks like TLS");
    }

    let client_stream = tokio::io::BufReader::new(stream);

    debug!("Established tunnel with {}", requested_addr);
    let uri = requested_addr.parse::<Uri>().unwrap();
    let host = uri.host().unwrap_or("localhost");

    debug!("Creating TLS acceptor for host: {}", host);
    let (leaf, key_pair) = ca
        .sign_leaf(host)
        .map_err(|e| io::Error::other(format!("Failed to sign leaf certificate: {e}")))?;

    // {
    //     let mut guard = flow.write().await;
    //     guard.leaf = Some(leaf.der().to_vec().into());
    // }
    // let mut roots = RootCertStore::empty();
    // roots.add(ca.ca_der.clone()).unwrap();
    // // Construct a fresh verifier using the test PKI roots, and the updated CRL.
    // let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
    //     .build()
    //     .unwrap();

    let mut tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::try_from(key_pair.serialize_der()).unwrap(),
        )
        .unwrap();

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    // tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    debug!("Creating TLS acceptor for client stream");
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let client_tls = acceptor
        .accept(client_stream)
        .await
        .map_err(|e| io::Error::other(format!("Client TLS handshake failed: {e}")))?;

    // TODO: add this information to flow
    // let tls_session = client_tls.get_ref().1;
    // let tls_version = tls_session.protocol_version();
    // let cipher_suite = tls_session.negotiated_cipher_suite();
    // let sni = tls_session.server_name();

    let alpn_bytes = client_tls.get_ref().1.alpn_protocol();
    let alpn = AlpnProtocol::from_bytes(alpn_bytes);

    match alpn {
        AlpnProtocol::Http2 => {
            return handle_h2(
                socket_addr,
                client_tls,
                requested_addr,
                flow_store,
                script_engine,
            )
            .await;
        }
        AlpnProtocol::Http1 => {
            info!("Using ALPN protocol: http/1.1");
        }
        AlpnProtocol::Http3 => {
            info!("Using ALPN protocol: http/3");
            // TODO: coming soon
        }
        AlpnProtocol::Unknown => {
            info!("No ALPN protocol negotiated, defaulting to http/1.1");
        }
    }

    let (peekable, bytes) = PeekableDuplex::new(client_tls, 1024).await?;

    let preview = std::str::from_utf8(&bytes).unwrap_or_default();

    if preview.contains("Upgrade: websocket") {
        return handle_wss(socket_addr, peekable, requested_addr, flow_store).await;
    }

    return handle_https(
        socket_addr,
        peekable,
        requested_addr,
        script_engine,
        flow_store,
    )
    .await;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpnProtocol {
    Http1,
    Http2,
    Http3,
    Unknown,
}

impl AlpnProtocol {
    pub fn from_bytes(alpn: Option<&[u8]>) -> Self {
        info!(
            "alpn {}",
            alpn.map(|a| String::from_utf8_lossy(a).to_string())
                .unwrap_or("idklol".to_string())
        );
        match alpn {
            Some(b"h3") => AlpnProtocol::Http3,
            Some(b"h2") => AlpnProtocol::Http2,
            Some(b"http/1.1") => AlpnProtocol::Http1,
            _ => AlpnProtocol::Unknown,
        }
    }
}
