use bytes::Bytes;
use http::StatusCode;
use http_body_util::Empty;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use roxy_shared::RoxyCA;
use roxy_shared::alpn::AlpnProtocol;
use roxy_shared::alpn::alp_h1_h2;
use roxy_shared::cert::ServerTlsConnectionData;
use roxy_shared::http::HttpError;
use roxy_shared::tls::RustlsServerConfig;
use roxy_shared::tls::TlsConfig;
use roxy_shared::uri::RUri;
use rustls::sign::CertifiedKey;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::trace;

use rustls::pki_types::PrivateKeyDer;

type ServerBuilder = hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response};
use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

use crate::flow::FlowCerts;
use crate::flow_store::FlowStore;
use crate::h3::start_h3;
use crate::http::handle_h2;
use crate::http::{handle_http, handle_https};
use crate::interceptor::ScriptEngine;
use crate::peek_stream::PeekStream;
use crate::utils::bad_connect_response;
use crate::utils::validate_connect_uri;
use crate::ws::{handle_ws, handle_wss};

const GET_BYTES: &[u8] = b"GET ";

#[derive(Debug, Clone)]
pub struct ProxyManager {
    port_tcp: u16,
    port_udp: u16,
    roxy_ca: RoxyCA,
    script_engine: ScriptEngine,
    tls_config: TlsConfig,
    pub flow_store: FlowStore,
    http_handle: Option<Arc<JoinHandle<()>>>,
    h3_handle: Option<Arc<JoinHandle<()>>>,
}

impl ProxyManager {
    pub fn new(
        port: u16,
        ca: RoxyCA,
        script_engine: ScriptEngine,
        tls_config: TlsConfig,
        flow_store: FlowStore,
    ) -> Self {
        ProxyManager {
            port_tcp: port,
            port_udp: port,
            roxy_ca: ca,
            script_engine,
            tls_config,
            flow_store,
            http_handle: None,
            h3_handle: None,
        }
    }

    pub async fn start_all(&mut self) -> Result<(), HttpError> {
        let tcp_listener =
            TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], self.port_tcp))).await?;
        let udp_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], self.port_udp)))?;

        let http_handle = start_tcp(self.cxt(), tcp_listener).await.map_err(|error| {
            HttpError::Io(std::io::Error::other(format!(
                "Failed to start TCP listener: {error}"
            )))
        })?;

        let h3_handle = start_h3(self.cxt(), udp_socket).await.map_err(|error| {
            HttpError::Io(std::io::Error::other(format!(
                "Failed to start H3 listener: {error:?}"
            )))
        })?;
        self.h3_handle = Some(Arc::new(h3_handle));
        self.http_handle = Some(Arc::new(http_handle));

        Ok(())
    }

    fn cxt(&self) -> ProxyContext {
        ProxyContext {
            roxy_ca: self.roxy_ca.clone(),
            script_engine: self.script_engine.clone(),
            flow_store: self.flow_store.clone(),
            tls_config: self.tls_config.clone(),
        }
    }

    pub async fn start_udp(&mut self, udp_socket: UdpSocket) -> Result<(), HttpError> {
        let addr = udp_socket.local_addr()?;
        let h3_handle = start_h3(self.cxt(), udp_socket)
            .await
            .map_err(|_| HttpError::Alpn)?; // TODO: Wrong error

        self.port_udp = addr.port();
        self.h3_handle = Some(Arc::new(h3_handle));

        Ok(())
    }
    pub async fn start_tcp(&mut self, tcp_listeneter: TcpListener) -> Result<(), HttpError> {
        let addr = tcp_listeneter.local_addr()?;
        let http_handle = start_tcp(self.cxt(), tcp_listeneter).await?;

        self.port_tcp = addr.port();
        self.http_handle = Some(Arc::new(http_handle));

        Ok(())
    }
}

impl Drop for ProxyManager {
    fn drop(&mut self) {
        if let Some(http_handle) = &self.http_handle {
            http_handle.abort();
        }
        if let Some(h3_handle) = &self.h3_handle {
            h3_handle.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlowContext {
    pub proxy_cxt: ProxyContext,
    pub client_addr: SocketAddr,
    pub target_uri: RUri,
    pub certs: FlowCerts,
}

impl FlowContext {
    pub fn new(client_addr: SocketAddr, target_uri: RUri, proxy_cxt: ProxyContext) -> Self {
        FlowContext {
            proxy_cxt,
            client_addr,
            target_uri,
            certs: FlowCerts::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyContext {
    pub roxy_ca: RoxyCA,
    pub script_engine: ScriptEngine,
    pub flow_store: FlowStore,
    pub tls_config: TlsConfig,
}

impl ProxyContext {
    pub fn new_flow(&self, client_addr: SocketAddr, target_uri: RUri) -> FlowContext {
        FlowContext::new(client_addr, target_uri, self.clone())
    }
    pub fn new_flow_upgrade(&self, client_addr: SocketAddr, target_uri: RUri) -> FlowContext {
        FlowContext::new(client_addr, target_uri, self.clone())
    }
}

async fn start_tcp(
    proxy_context: ProxyContext,
    tcp_listeneter: TcpListener,
) -> Result<JoinHandle<()>, HttpError> {
    let addr = tcp_listeneter.local_addr()?;
    let handle = tokio::spawn(async move {
        trace!("TCP listening on {addr}");
        while let Ok((stream, addr)) = tcp_listeneter.accept().await {
            let cxt = proxy_context.clone();
            tokio::task::spawn(async move {
                if let Err(err) = ServerBuilder::new()
                    .title_case_headers(true)
                    .serve_connection(
                        TokioIo::new(stream),
                        service_fn(|req| proxy(cxt.clone(), addr, req)),
                    )
                    .with_upgrades()
                    .await
                {
                    error!("Failed to serve connection: {:?}", err);
                }
            });
        }
        error!("TCP proxy finished");
    });
    Ok(handle)
}

async fn proxy(
    proxy_context: ProxyContext,
    socket_addr: SocketAddr,
    incoming_request: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, HttpError> {
    let connection_id = proxy_context.flow_store.new_connection(socket_addr).await;
    if Method::CONNECT == incoming_request.method() {
        trace!("CONNECT request: {:?}", incoming_request.uri());
        if !validate_connect_uri(
            incoming_request.version(),
            incoming_request.uri(),
            incoming_request.headers(),
        ) {
            proxy_context
                .flow_store
                .post_proxy_event(crate::flow_store::ConnectionEvent::failure(
                    connection_id,
                    "Validate correct failed",
                ));
            debug!("Invalid connect request");
            return bad_connect_response().map_err(|_| HttpError::ProxyConnect);
        }

        let flow_context = FlowContext::new(
            socket_addr,
            RUri::new(incoming_request.uri().clone()),
            proxy_context.clone(),
        );
        tokio::spawn(async {
            match hyper::upgrade::on(incoming_request).await {
                Ok(upgraded) => {
                    if let Err(error) = tunnel(flow_context, upgraded).await {
                        error!("server io error: {}", error);
                    };
                }
                Err(e) => {
                    error!("upgrade error: {}", e);
                }
            }
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(BoxBody::new(Empty::<Bytes>::new()))?)
    } else {
        handle_http(
            FlowContext::new(socket_addr, incoming_request.uri().into(), proxy_context),
            incoming_request,
        )
        .await
    }
}

async fn tunnel(
    mut flow_cxt: FlowContext,
    upgraded: Upgraded,
) -> Result<(), Box<dyn std::error::Error>> {
    trace!("Providing tunnel");
    let client_stream = TokioIo::new(upgraded);

    let (client_stream, peeked_bytes) = PeekStream::new(client_stream, 1024).await?;

    // GET request indicates this is a websocket connection, not TLS
    if peeked_bytes.starts_with(GET_BYTES) {
        return handle_ws(flow_cxt, client_stream).await;
    }
    trace!("Peek looks like TLS");

    let (leaf, key_pair) = flow_cxt
        .proxy_cxt
        .roxy_ca
        .sign_leaf_uri(&flow_cxt.target_uri)
        .map_err(|e| io::Error::other(format!("Failed to sign leaf certificate: {e}")))?;

    let pk_der = PrivateKeyDer::try_from(key_pair.serialize_der())?;
    let provider = flow_cxt.proxy_cxt.tls_config.crypto_provider();
    let certified_key = CertifiedKey::from_der(vec![leaf.der().clone()], pk_der, provider.deref())?;

    let RustlsServerConfig {
        resolver,
        mut server_config,
    } = flow_cxt
        .proxy_cxt
        .tls_config
        .rustls_server_config(certified_key)?;

    server_config.alpn_protocols = alp_h1_h2();

    trace!("Creating TLS acceptor for client stream");
    let client_tls = TlsAcceptor::from(Arc::new(server_config))
        .accept(client_stream)
        .await
        .map_err(|e| io::Error::other(format!("Client TLS handshake failed: {e}")))?;

    let client_hello = resolver
        .client_hello
        .lock()
        .map_err(|e| io::Error::other(format!("failed to gain lock on resolver {e}")))?
        .to_owned();

    let client_tls_session: ServerTlsConnectionData = client_tls.get_ref().1.into();
    let alpn = client_tls_session.alpn.clone();

    flow_cxt.certs.client_hello = client_hello;
    flow_cxt.certs.client_tls = Some(client_tls_session);

    match alpn {
        AlpnProtocol::Http2 => handle_h2(flow_cxt, client_tls).await,
        AlpnProtocol::Http1 => {
            trace!("Using ALPN protocol: http/1.1");
            let (peekable, bytes) = PeekStream::new(client_tls, 1024).await?;
            let preview = std::str::from_utf8(&bytes).unwrap_or_default();
            if preview.contains("Upgrade: websocket") {
                handle_wss(flow_cxt, peekable).await
            } else {
                handle_https(flow_cxt, peekable).await
            }
        }
        AlpnProtocol::Unknown(alpn_bytes) => {
            trace!(
                "No ALPN protocol negotiated {alpn_bytes:?}, defaulting to http/2 which can downgrade"
            );

            handle_h2(flow_cxt, client_tls).await
        }
        AlpnProtocol::Http3 => {
            error!("H3 negotiated over TCP, reverting to http/1.1");
            Err(Box::new(HttpError::Alpn)) // TODO: make secific
        }
        AlpnProtocol::None => {
            error!("No alpn negotiated");
            handle_https(flow_cxt, client_tls).await
        }
    }
}
