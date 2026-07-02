use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;

use roxy_shared::http::HttpEmitter;
use roxy_shared::http::HttpEvent;

use once_cell::sync::Lazy;
use snowflake::SnowflakeIdGenerator;
use time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock, watch};
use tracing::error;
use tracing::warn;

use crate::flow::{Flow, FlowConnection, InterceptedRequest, InterceptedResponse, WsMessage};
use crate::proxy::FlowContext;

static ID_GENERATOR: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let generator = SnowflakeIdGenerator::new(1, 1);
    Mutex::new(generator)
});

async fn next_id() -> i64 {
    ID_GENERATOR.lock().await.generate()
}

#[derive(Debug, Clone)]
pub struct ProxyConnection {
    pub id: i64,
    pub time_stamp: OffsetDateTime,
    pub socket_addr: SocketAddr,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FlowStore {
    pub flows: Arc<DashMap<i64, Arc<RwLock<Flow>>>>,
    pub ordered_ids: Arc<RwLock<Vec<i64>>>,
    pub notifier: watch::Sender<()>,
    pub notifier_new_flow: watch::Sender<()>,
    pub event_tx: UnboundedSender<ProxyEvent>,
    pub connections: Arc<DashMap<i64, Arc<RwLock<ProxyConnection>>>>,
    pub connections_ordered_ids: Arc<RwLock<Vec<i64>>>,
    pub connections_notifier: watch::Sender<()>,
}

impl FlowStore {
    pub fn new() -> Self {
        let (notifier, _) = watch::channel(());
        let (notifier_new_flow, _) = watch::channel(()); // TODO: write this
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (connection_notifier, _) = watch::channel(());
        let out = Self {
            flows: Arc::new(DashMap::new()),
            ordered_ids: Arc::new(RwLock::new(Vec::new())),
            notifier,
            notifier_new_flow,
            event_tx,
            connections: Arc::new(DashMap::new()),
            connections_ordered_ids: Arc::new(RwLock::new(Vec::new())),
            connections_notifier: connection_notifier,
        };

        out.event_proc(event_rx);
        out
    }

    pub async fn new_connection(&self, socket_addr: SocketAddr) -> i64 {
        let id = next_id().await;
        let proxy_connection = Arc::new(RwLock::new(ProxyConnection {
            id,
            time_stamp: OffsetDateTime::now_utc(),
            socket_addr,
            message: None,
        }));
        self.connections.insert(id, proxy_connection);
        self.connections_ordered_ids.write().await.push(id);
        self.connections_notifier.send(()).unwrap_or_else(|_| {
            error!("Failed to notify subscribers, channel closed");
        });
        self.notify();
        id
    }

    #[allow(clippy::panic)]
    pub async fn new_flow_cxt(&self, cxt: &FlowContext) -> i64 {
        let flow_id = next_id().await;

        let connection = self
            .connections
            .get(&cxt.client_connection_id)
            .unwrap_or_else(|| panic!("Invalid connection id {}", cxt.client_connection_id));

        let mut flow = Flow::new(
            flow_id,
            FlowConnection {
                connection_id: cxt.client_connection_id,
                addr: cxt.client_addr,
            },
            None,
        );

        flow.timing.client_conn_established = Some(connection.read().await.time_stamp);
        flow.certs = cxt.certs.clone();

        let flow = Arc::new(RwLock::new(flow));
        self.flows.insert(flow_id, flow);
        self.ordered_ids.write().await.push(flow_id);
        self.notify();
        flow_id
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
    pub fn post_proxy_event(&self, proxy_event: ProxyEvent) {
        if let Err(err) = self.event_tx.send(proxy_event) {
            error!("Error posting event {err}");
        }
    }
    pub fn post_event(&self, flow_id: i64, event: FlowEventKind) {
        let fe = FlowEvent::new(flow_id, event);
        let pe = ProxyEvent::Flow(fe);
        if let Err(err) = self.event_tx.send(pe) {
            error!("Error posting event {err}");
        }
    }

    pub fn post_event_fe(&self, event: FlowEvent) {
        let pe = ProxyEvent::Flow(event);
        if let Err(err) = self.event_tx.send(pe) {
            error!("Error posting event {err}");
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
    fn event_proc(&self, mut event_rx: UnboundedReceiver<ProxyEvent>) {
        let flow_store = self.clone();
        tokio::spawn(async move {
            while let Some(proxy_event) = event_rx.recv().await {
                match proxy_event {
                    ProxyEvent::Flow(flow_event) => {
                        let flow = flow_store
                            .flows
                            .get(&flow_event.id)
                            .expect("FlowId not in map {flow_id}");
                        let mut guard = flow.write().await;
                        match flow_event.kind {
                            FlowEventKind::HttpEvent(inner) => match inner {
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
                                HttpEvent::ServerTlsConn(
                                    _server_tls_conn,
                                    _client_verification,
                                ) => {
                                    // TODO: this is captured earlier in the flow
                                    // guard.certs.client_tls = Some(server_tls_conn);
                                    // guard.certs.client_verification = Some(client_verification);
                                }
                                HttpEvent::ServerTlsConnInitiated => {
                                    guard.timing.server_conn_tls_initiated =
                                        Some(OffsetDateTime::now_utc())
                                }
                                HttpEvent::ClientTlsHandshake => {
                                    guard.timing.client_conn_tls_handshake =
                                        Some(OffsetDateTime::now_utc());
                                }
                                HttpEvent::ServerConnInitiated => {
                                    guard.timing.server_conn_initiated =
                                        Some(OffsetDateTime::now_utc());
                                }
                                HttpEvent::ServerConnClosed => {
                                    guard.timing.server_conn_closed =
                                        Some(OffsetDateTime::now_utc());
                                }
                            },
                            FlowEventKind::Response(resp) => {
                                guard.response = Some(resp);
                            }
                            FlowEventKind::WsMessage(wsm) => {
                                guard.messages.push(wsm);
                            }
                            FlowEventKind::Error(error) => {
                                guard.error.replace(error);
                            }
                            FlowEventKind::Request(intercepted_request) => {
                                guard.request = Some(intercepted_request);
                            }
                        }
                    }

                    ProxyEvent::Connection(connection_event) => {
                        let guard = flow_store
                            .connections
                            .get(&connection_event.id)
                            .expect("ConnectionId not in map {connection_event.id}");
                        match connection_event.kind {
                            ConnectionEventKind::ConnectionFailure(reason) => {
                                guard.write().await.message = Some(reason.clone());
                            }
                            ConnectionEventKind::ConnectionEnded => {
                                guard.write().await.message = Some("Close".to_string());
                            }
                        }
                        flow_store
                            .connections_notifier
                            .send(())
                            .unwrap_or_else(|_| {
                                warn!("Failed to notify subscribers, channel closed");
                            });
                    }
                }

                flow_store.notify();
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
    fn emit(&self, event: HttpEvent) {
        self.flow_store
            .post_event_fe(FlowEvent::new(self.id, FlowEventKind::HttpEvent(event)));
    }
}
#[derive(Debug)]
pub enum ProxyEvent {
    Connection(ConnectionEvent),
    Flow(FlowEvent),
}

#[derive(Debug)]
pub struct FlowEvent {
    id: i64,
    kind: FlowEventKind,
}
impl FlowEvent {
    pub fn new(id: i64, kind: FlowEventKind) -> Self {
        Self { id, kind }
    }
}

#[derive(Debug)]
pub enum FlowEventKind {
    Request(InterceptedRequest),
    Response(InterceptedResponse),
    WsMessage(WsMessage),
    HttpEvent(HttpEvent),
    Error(String),
}

#[derive(Debug)]
pub struct ConnectionEvent {
    id: i64,
    kind: ConnectionEventKind,
}

impl ConnectionEvent {
    pub fn failure(id: i64, message: &str) -> ProxyEvent {
        ProxyEvent::Connection(Self {
            id,
            kind: ConnectionEventKind::ConnectionFailure(message.to_string()),
        })
    }
}

#[derive(Debug)]
pub enum ConnectionEventKind {
    ConnectionFailure(String),
    ConnectionEnded,
}

impl Default for FlowStore {
    fn default() -> Self {
        Self::new()
    }
}
