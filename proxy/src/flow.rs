use std::{net::SocketAddr, sync::Arc};

use dashmap::DashMap;

use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use http::{StatusCode, Version};
use roxy_shared::alpn::AlpnProtocol;

use roxy_shared::body::create_http_body;
use roxy_shared::cert::CapturedClientHello;
use roxy_shared::cert::CapturedResolveClientCert;
use roxy_shared::cert::ClientTlsConnectionData;
use roxy_shared::cert::ClientVerificationCapture;
use roxy_shared::cert::ServerTlsConnectionData;
use roxy_shared::cert::ServerVerificationCapture;
use roxy_shared::content::get_content_encoding;
use roxy_shared::content::{Encodings, decode_body};
use roxy_shared::http::{HttpEmitter, HttpEvent};
use roxy_shared::uri::RUri;
use roxy_shared::uri::Scheme;

use http::HeaderMap;
use once_cell::sync::Lazy;
use roxy_shared::body::BytesBody;
use roxy_shared::version::HttpVersion;
use snowflake::SnowflakeIdGenerator;
use time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::error;
use tracing::warn;

use crate::proxy::FlowContext;

static ID_GENERATOR: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let generator = SnowflakeIdGenerator::new(1, 1);
    Mutex::new(generator)
});

async fn next_id() -> i64 {
    ID_GENERATOR.lock().await.generate()
}

#[derive(Debug, Clone)]
pub struct FlowStore {
    pub flows: Arc<DashMap<i64, Arc<RwLock<Flow>>>>,
    pub ordered_ids: Arc<RwLock<Vec<i64>>>,
    pub notifier: watch::Sender<()>,
    pub notifier_new_flow: watch::Sender<()>,
    pub event_tx: UnboundedSender<(i64, FlowEvent)>,
}

impl FlowStore {
    pub fn new() -> Self {
        let (notifier, _) = watch::channel(());
        let (notifier_new_flow, _) = watch::channel(()); // TODO: write this
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let s = Self {
            flows: Arc::new(DashMap::new()),
            ordered_ids: Arc::new(RwLock::new(Vec::new())),
            notifier,
            notifier_new_flow,
            event_tx,
        };

        s.event_proc(event_rx);
        s
    }

    pub async fn new_flow_cxt(&self, cxt: &FlowContext, req: InterceptedRequest) -> i64 {
        let id = next_id().await;
        let mut flow = Flow::new(
            id,
            FlowConnection {
                addr: cxt.client_addr,
            },
            Some(req),
        );

        flow.certs = cxt.certs.clone();

        let flow = Arc::new(RwLock::new(flow));
        self.flows.insert(id, flow.clone());
        self.ordered_ids.write().await.push(id);
        self.notify();
        id
    }

    pub async fn new_ws_flow(&self, client_connect: FlowConnection) -> i64 {
        let id = next_id().await;
        let flow = Arc::new(RwLock::new(Flow::new(id, client_connect, None)));
        self.flows.insert(id, flow.clone());
        self.ordered_ids.write().await.push(id);
        self.notify();
        id
    }

    pub async fn get_flow_by_id(&self, id: i64) -> Option<Arc<RwLock<Flow>>> {
        self.flows.get(&id).map(|f| f.value().clone())
    }

    pub fn post_event(&self, flow_id: i64, event: FlowEvent) {
        if let Err(err) = self.event_tx.send((flow_id, event)) {
            error!("Error posting event {err} {flow_id}");
        }
    }

    fn notify(&self) {
        self.notifier.send(()).unwrap_or_else(|_| {
            warn!("Failed to notify subscribers, channel closed");
        });
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.notifier.subscribe()
    }

    #[allow(clippy::expect_used)]
    fn event_proc(&self, mut event_rx: UnboundedReceiver<(i64, FlowEvent)>) {
        let fs = self.clone();
        tokio::spawn(async move {
            while let Some((flow_id, event)) = event_rx.recv().await {
                let flow = fs.flows.get(&flow_id).expect("FlowId not in map {flow_id}");

                let mut guard = flow.write().await;
                match event {
                    FlowEvent::HttpEvent(inner) => match inner {
                        HttpEvent::TcpConnect(addr) => {
                            guard.server_connection = Some(FlowConnection { addr });
                            guard.timing.server_conn_tcp_handshake =
                                Some(OffsetDateTime::now_utc());
                        }
                        HttpEvent::ClientHttpHandshakeStart => {
                            guard.timing.server_conn_http_handshake =
                                Some(OffsetDateTime::now_utc());
                        }
                        HttpEvent::ClientHttpHandshakeComplete => {}
                        HttpEvent::ClientTlsConn(tls_conn_data, server_verification) => {
                            guard.certs.server_tls = Some(tls_conn_data);
                            guard.certs.server_verification = Some(server_verification);
                            guard.timing.server_conn_tls_handshake =
                                Some(OffsetDateTime::now_utc());
                        }
                        HttpEvent::ServerTlsConn(_server_tls_conn, _client_verification) => {
                            // TODO: this is captured earlier in the flow
                            // guard.certs.client_tls = Some(server_tls_conn);
                            // guard.certs.client_verification = Some(client_verification);
                        }
                        HttpEvent::ServerTlsConnInitiated => {
                            guard.timing.server_conn_tls_initiated = Some(OffsetDateTime::now_utc())
                        }
                        HttpEvent::ClientTlsHandshake => {
                            guard.timing.client_conn_tls_handshake =
                                Some(OffsetDateTime::now_utc());
                        }
                    },
                    FlowEvent::Response(resp) => {
                        guard.response = Some(resp);
                    }
                    FlowEvent::WsMessage(wsm) => {
                        guard.messages.push(wsm);
                    }
                }
                drop(guard);

                fs.notify();
            }
        });
    }
}

#[derive(Debug)]
pub struct FlowEventEmitter {
    id: i64,
    flow_store: FlowStore,
}

impl FlowEventEmitter {
    pub fn new(id: i64, flow_store: FlowStore) -> Self {
        Self { id, flow_store }
    }
}

impl HttpEmitter for FlowEventEmitter {
    fn emit(&self, event: roxy_shared::http::HttpEvent) {
        self.flow_store
            .post_event(self.id, FlowEvent::HttpEvent(event));
    }
}

#[derive(Debug)]
pub enum FlowEvent {
    Response(InterceptedResponse),
    WsMessage(WsMessage),
    HttpEvent(HttpEvent),
}

impl Default for FlowStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Flow {
    pub id: i64,
    pub timing: Timing,

    pub client_connection: FlowConnection,
    pub request: Option<InterceptedRequest>,

    pub server_connection: Option<FlowConnection>,
    pub response: Option<InterceptedResponse>,

    pub error: Option<String>,

    pub certs: FlowCerts,

    pub messages: Vec<WsMessage>,
}

#[derive(Debug, Default, Clone)]
pub struct FlowCerts {
    pub client_hello: Option<CapturedClientHello>,
    pub client_verification: Option<ClientVerificationCapture>,
    pub client_tls: Option<ServerTlsConnectionData>,

    pub server_resolve_client_cert: Option<CapturedResolveClientCert>,
    pub server_verification: Option<ServerVerificationCapture>,
    pub server_tls: Option<ClientTlsConnectionData>,
}

#[derive(Debug, Clone, Copy)]
pub struct FlowConnection {
    pub addr: SocketAddr,
}

impl Flow {
    fn new(
        id: i64,
        client_connection: FlowConnection,
        request: Option<InterceptedRequest>,
    ) -> Self {
        Self {
            id,
            timing: Timing::default(),
            client_connection,
            server_connection: None,
            request,
            response: None,
            certs: FlowCerts::default(),
            error: None,
            messages: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct WsMessage {
    pub message: Message,
    pub direction: WsDirection,
    pub timestamp: OffsetDateTime,
}

impl WsMessage {
    pub fn client(message: Message) -> Self {
        Self {
            message,
            direction: WsDirection::Client,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
    pub fn server(message: Message) -> Self {
        Self {
            message,
            direction: WsDirection::Server,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsDirection {
    Client,
    Server,
}

#[derive(Debug, Default, Clone)]
pub struct TlsMetadata {
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub negotiated_cipher: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Timing {
    pub client_conn_established: Option<OffsetDateTime>,
    pub client_conn_tls_handshake: Option<OffsetDateTime>,

    pub server_conn_initiated: Option<OffsetDateTime>,
    pub server_conn_tcp_handshake: Option<OffsetDateTime>,

    pub server_conn_tls_initiated: Option<OffsetDateTime>,
    pub server_conn_tls_handshake: Option<OffsetDateTime>,

    pub server_conn_http_handshake: Option<OffsetDateTime>,

    pub first_request_bytes: Option<OffsetDateTime>,
    pub request_complete: Option<OffsetDateTime>,

    pub first_response_bytes: Option<OffsetDateTime>,
    pub response_complete: Option<OffsetDateTime>,

    pub client_conn_closed: Option<OffsetDateTime>,
    pub server_conn_closed: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptedRequest {
    pub timestamp: OffsetDateTime,
    pub uri: RUri,
    pub encoding: Option<Vec<Encodings>>,
    pub alpn: AlpnProtocol,
    pub method: http::Method,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub body: bytes::Bytes,
    pub trailers: Option<HeaderMap>,
}

impl Default for InterceptedRequest {
    fn default() -> Self {
        Self {
            timestamp: OffsetDateTime::now_utc(),
            uri: RUri::default(),
            encoding: None,
            alpn: AlpnProtocol::None,
            method: http::Method::GET,
            version: HttpVersion(Version::HTTP_11),
            headers: HeaderMap::new(),
            body: bytes::Bytes::new(),
            trailers: None,
        }
    }
}

impl InterceptedRequest {
    pub fn from_http(
        uri: RUri,
        alpn: AlpnProtocol,
        parts: http::request::Parts,
        body_bytes: bytes::Bytes,
        trailers: Option<HeaderMap>,
    ) -> Self {
        let encoding = get_content_encoding(&parts.headers);

        let body = match encoding.clone() {
            Some(enc) => match decode_body(&body_bytes, &enc) {
                Ok(body) => body,
                Err(e) => {
                    warn!("Failed to decode body encoding  err: '{e}'");
                    body_bytes
                }
            },
            None => body_bytes,
        };
        let mut headers = parts.headers;
        headers.remove(CONTENT_LENGTH);
        headers.remove(TRANSFER_ENCODING);

        InterceptedRequest {
            timestamp: OffsetDateTime::now_utc(),
            uri: uri.clone(),
            encoding,
            alpn,
            method: parts.method,
            version: parts.version.into(),
            headers,
            body,
            trailers,
        }
    }

    pub fn scheme(&self) -> Scheme {
        if self.uri.scheme_str().is_some() {
            return self.uri.scheme();
        }
        if self.alpn.is_tls() {
            Scheme::Https
        } else {
            Scheme::Http
        }
    }

    pub fn line_pretty(&self) -> String {
        self.uri.inner.to_string()
    }

    pub fn request_builder(&self) -> http::request::Builder {
        let parts = format!(
            "{}://{}:{}{}",
            self.uri.scheme(),
            self.uri.host(),
            self.uri.port(),
            self.uri.path_and_query()
        );

        let mut builder = http::Request::builder()
            .method(self.method.clone())
            .uri(parts)
            .version(self.version.0);

        for (key, value) in self.headers.iter() {
            builder = builder.header(key, value);
        }
        builder
    }

    pub fn request(&self) -> Result<http::Request<BytesBody>, http::Error> {
        self.request_builder().body(create_http_body(
            self.body.clone(),
            self.encoding.clone(),
            self.trailers.clone(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptedResponse {
    pub timestamp: OffsetDateTime,
    pub status: StatusCode,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub encoding: Option<Vec<Encodings>>,
    pub body: bytes::Bytes,
    pub trailers: Option<HeaderMap>,
}

impl Default for InterceptedResponse {
    fn default() -> Self {
        Self {
            timestamp: OffsetDateTime::now_utc(),
            status: StatusCode::OK,
            version: HttpVersion(Version::HTTP_11),
            headers: HeaderMap::new(),
            encoding: None,
            body: bytes::Bytes::new(),
            trailers: None,
        }
    }
}

impl InterceptedResponse {
    pub fn from_http(
        parts: http::response::Parts,
        body_bytes: bytes::Bytes,
        trailers: Option<HeaderMap>,
    ) -> Self {
        let encoding = get_content_encoding(&parts.headers);
        let body = match &encoding {
            Some(enc) => match decode_body(&body_bytes, enc) {
                Ok(body) => body,
                Err(e) => {
                    warn!("Failed to decode body encoding err: '{e}'");
                    body_bytes
                }
            },
            None => body_bytes,
        };

        let mut headers = parts.headers;
        headers.remove(CONTENT_LENGTH);
        headers.remove(TRANSFER_ENCODING);

        InterceptedResponse {
            timestamp: OffsetDateTime::now_utc(),
            status: parts.status,
            version: parts.version.into(),
            headers,
            encoding,
            body,
            trailers,
        }
    }

    pub fn request_line(&self) -> String {
        format!("{:?} {}", self.version, self.status)
    }

    pub fn response_builder(&self) -> http::response::Builder {
        let mut builder = http::Response::builder()
            .status(self.status)
            .version(self.version.0);

        for (key, value) in self.headers.iter() {
            builder = builder.header(key, value)
        }
        builder
    }

    pub fn response(&self) -> Result<http::Response<BytesBody>, http::Error> {
        let builder = self.response_builder();

        builder.body(create_http_body(
            self.body.clone(),
            self.encoding.clone(),
            self.trailers.clone(),
        ))
    }
}
