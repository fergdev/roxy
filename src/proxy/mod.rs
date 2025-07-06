mod cert;
mod h1;
mod h2;
mod peekable_duplex;
mod ws;

use chrono::Utc;
use h1::{handle_http, handle_https};
use h2::handle_h2;
use http::Uri;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use peekable_duplex::PeekableDuplex;
use roxy_shared::RoxyCA;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use rustls::pki_types::PrivateKeyDer;
use rustls::server::ServerConfig;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::Mutex;

type ServerBuilder = hyper::server::conn::http1::Builder;
use bytes::Bytes;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response};
use tokio_rustls::TlsAcceptor;
use ws::{handle_ws, handle_wss};

use std::sync::Arc;

use crate::flow::{Flow, FlowStore, InterceptedRequest, Scheme};
use crate::interceptor::ScriptEngine;
use crate::notify_info;

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

    {
        let mut flow = flow.write().await;
        flow.timing.client_conn_established = Some(Utc::now());
    }

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
        body: body_bytes.clone()
    };

    if Method::CONNECT == parts.method {
        debug!("CONNECT request: {:?}", parts.uri);
        if let Some(addr) = host_addr(&parts.uri) {
            let req =
                Request::from_parts(parts, http_body_util::Full::new(body_bytes.clone()).boxed());
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) =
                            tunnel(flow, upgraded, &addr, intercepted, ca, script_engine, fs).await
                        {
                            error!("server io error: {}", e);
                        };
                    }
                    Err(e) => {
                        error!("upgrade error: {}", e);
                    }
                }
            });

            Ok(Response::new(empty()))
        } else {
            debug!("CONNECT host is not socket addr: {:?}", parts.uri);

            // TODO: write error
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        handle_http(intercepted, flow, body_bytes, script_engine, fs).await
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
    requested_addr: &str,
    connect: InterceptedRequest,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> std::io::Result<()> {
    let client_stream = TokioIo::new(upgraded);

    // TODO: Peek less here
    let (stream, peeked_bytes) = PeekableDuplex::new(client_stream, 1024).await?;

    if peeked_bytes.starts_with(b"GET ") {
        return handle_ws(stream, requested_addr, flow, connect, flow_store).await;
    } else if peeked_bytes.starts_with(&[0x16]) {
        info!("Looks like TLS");
    }

    let client_stream = tokio::io::BufReader::new(stream);

    debug!("Established tunnel with {}", requested_addr);
    let uri = requested_addr.parse::<Uri>().unwrap();
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16().unwrap_or(443);

    debug!("Creating TLS acceptor for host: {}", host);
    let leaf = ca.sign_leaf(host).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to sign leaf certificate: {}", e),
        )
    })?;

    let mut tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::try_from(ca.key_pair.serialize_der()).unwrap(),
        )
        .unwrap();

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    debug!("Creating TLS acceptor for client stream");
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let client_tls = acceptor.accept(client_stream).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("TLS handshake failed: {}", e),
        )
    })?;

    let alpn_bytes = client_tls.get_ref().1.alpn_protocol();
    let alpn = AlpnProtocol::from_bytes(alpn_bytes);

    match alpn {
        AlpnProtocol::Http2 => {
            return handle_h2(client_tls, requested_addr, flow, connect, flow_store).await;
        }
        AlpnProtocol::Http1 => {
            info!("Using ALPN protocol: http/1.1");
            notify_info!("Using ALPN protocol: http/1.1");
        }
        AlpnProtocol::Http3 => {
            info!("Using ALPN protocol: http/3");
            // TODO: coming soon
        }
        AlpnProtocol::Unknown => {
            notify_info!("No ALPN protocol negotiated, defaulting to http/1.1");
            info!("No ALPN protocol negotiated, defaulting to http/1.1");
        }
    }

    let (peekable, bytes) = PeekableDuplex::new(client_tls, 1024).await?;

    let preview = std::str::from_utf8(&bytes).unwrap_or_default();

    if preview.contains("Upgrade: websocket") {
        return handle_wss(peekable, requested_addr, flow, connect, flow_store).await;
    }

    return handle_https(
        peekable,
        requested_addr,
        connect,
        flow,
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
        match alpn {
            Some(b"h2") => AlpnProtocol::Http2,
            Some(b"http/1.1") => AlpnProtocol::Http1,
            Some(b"h3") => AlpnProtocol::Http3,
            _ => AlpnProtocol::Unknown,
        }
    }
}
