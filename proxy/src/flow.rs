use std::{collections::HashMap, fmt::Display, net::SocketAddr, sync::Arc};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use dashmap::DashMap;

use http::Uri;
use once_cell::sync::Lazy;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::{Mutex, RwLock, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

static ID_GENERATOR: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let generator = SnowflakeIdGenerator::new(1, 1);
    Mutex::new(generator)
});

pub async fn next_id() -> i64 {
    ID_GENERATOR.lock().await.generate()
}

#[derive(Clone)]
pub struct FlowStore {
    pub flows: Arc<DashMap<i64, Arc<RwLock<Flow>>>>,
    pub ordered_ids: Arc<RwLock<Vec<i64>>>,
    pub notifier: watch::Sender<()>,
}

impl FlowStore {
    pub fn new() -> Self {
        let (notifier, _) = watch::channel(());
        Self {
            flows: Arc::new(DashMap::new()),
            ordered_ids: Arc::new(RwLock::new(Vec::new())),
            notifier,
        }
    }
    pub async fn new_flow(&self, client_connect: FlowConnection) -> Arc<RwLock<Flow>> {
        let id = next_id().await;
        let flow = Arc::new(RwLock::new(Flow::new(id, client_connect)));
        self.flows.insert(id, flow.clone());
        self.ordered_ids.write().await.push(id);
        flow
    }
    pub async fn add_flow(&self, flow: Flow) {
        self.flows.insert(flow.id, Arc::new(RwLock::new(flow)));
        let _ = self.notifier.send(());
    }

    pub async fn get_flow_by_id(&self, id: i64) -> Option<Arc<RwLock<Flow>>> {
        self.flows.get(&id).map(|f| f.value().clone())
    }

    pub fn notify(&self) {
        info!("notify");
        self.notifier.send(()).unwrap_or_else(|_| {
            warn!("Failed to notify subscribers, channel closed");
        });
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.notifier.subscribe()
    }
}

impl Default for FlowStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Flow {
    pub id: i64,
    pub timing: Timing,
    pub client_connection: FlowConnection,
    pub server_connection: Option<FlowConnection>,
    pub connect: Option<InterceptedRequest>,
    pub error: Option<String>,
    pub leaf: Option<Bytes>,
    pub kind: FlowKind,
}

#[derive(Clone, Copy)]
pub struct FlowConnection {
    pub addr: SocketAddr,
}

impl Flow {
    pub fn new(id: i64, client_connection: FlowConnection) -> Self {
        Self {
            id,
            timing: Timing::default(),
            client_connection,
            server_connection: None,
            kind: FlowKind::Unknown,
            connect: None,
            error: None,
            leaf: None,
        }
    }
}

pub enum FlowKind {
    Http(HttpFlow),
    Https(HttpsFlow),
    Http2(Http2Flow),

    Ws(WsFlow),
    Wss(WssFlow),

    Unknown,
}

pub struct HttpFlow {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
}

impl HttpFlow {
    pub fn new(request: InterceptedRequest) -> Self {
        Self {
            request,
            response: None,
        }
    }
}

pub struct HttpsFlow {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub tls_metadata: Option<TlsMetadata>,
    pub cert_info: Vec<Bytes>,
}

impl HttpsFlow {
    pub fn new(request: InterceptedRequest) -> Self {
        Self {
            request,
            response: None,
            tls_metadata: None,
            cert_info: Vec::new(),
        }
    }
}

pub struct WsFlow {
    pub messages: Vec<WsMessage>,
}

impl WsFlow {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl Default for WsFlow {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WssFlow {
    pub messages: Vec<WsMessage>,
    pub cert_info: Vec<Bytes>,
    pub tls_metadata: Option<TlsMetadata>,
}

#[derive(Debug, Clone)]
pub struct WsMessage {
    pub message: Message,
    pub direction: WsDirection,
    pub timestamp: DateTime<Utc>,
}

impl WsMessage {
    pub fn client(message: Message) -> Self {
        Self {
            message,
            direction: WsDirection::Client,
            timestamp: Utc::now(),
        }
    }
    pub fn server(message: Message) -> Self {
        Self {
            message,
            direction: WsDirection::Server,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsDirection {
    Client,
    Server,
}

impl WssFlow {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            cert_info: Vec::new(),
            tls_metadata: None,
        }
    }
}

impl Default for WssFlow {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Http2Flow {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub cert_info: Vec<Bytes>,
    pub tls_metadata: Option<TlsMetadata>,
}

impl Http2Flow {
    pub fn new(req: InterceptedRequest) -> Self {
        Self {
            request: req,
            response: None,
            cert_info: Vec::new(),
            tls_metadata: None,
        }
    }
}

pub struct TlsMetadata {
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub negotiated_cipher: Option<String>,
}

#[derive(Default, Clone)]
pub struct Timing {
    pub client_conn_established: Option<DateTime<Utc>>,
    pub server_conn_initiated: Option<DateTime<Utc>>,
    pub server_conn_tcp_handshake: Option<DateTime<Utc>>,
    pub server_conn_tls_handshake: Option<DateTime<Utc>>,
    pub client_conn_tls_handshake: Option<DateTime<Utc>>,
    pub first_reques_byte: Option<DateTime<Utc>>,
    pub request_complet_: Option<DateTime<Utc>>,
    pub first_respons_byte: Option<DateTime<Utc>>,
    pub response_complet_: Option<DateTime<Utc>>,
    pub client_conn_closed: Option<DateTime<Utc>>,
    pub server_conn_closed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
}

impl Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Scheme::Http => "http".to_string(),
            Scheme::Https => "https".to_string(),
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct InterceptedRequest {
    pub timestamp: DateTime<Utc>,
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub version: u8,
    pub headers: HashMap<String, String>,
    pub body: bytes::Bytes,
}

impl InterceptedRequest {
    pub fn from_http(
        scheme: Scheme,
        parts: &http::request::Parts,
        host: &str,
        port: u16,
        body_bytes: &bytes::Bytes,
    ) -> Self {
        InterceptedRequest {
            timestamp: Utc::now(),
            scheme,
            host: parts.uri.host().unwrap_or(host).to_string(),
            port: parts.uri.port_u16().unwrap_or(port),
            path: parts.uri.path().to_string(),
            method: parts.method.to_string(),
            version: match parts.version {
                http::Version::HTTP_10 => 0,
                http::Version::HTTP_11 => 1,
                http::Version::HTTP_2 => 2,
                _ => 3,
            },
            headers: parts
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: body_bytes.clone(),
        }
    }

    pub fn uri(&self) -> Result<Uri, http::Error> {
        Uri::builder()
            .scheme(self.scheme.to_string().as_str())
            .authority(self.host.clone())
            .path_and_query(self.path.clone())
            .build()
    }
    pub fn uri_https(&self) -> Result<Uri, http::Error> {
        Uri::builder()
            .scheme("http")
            .authority(self.host.clone())
            .path_and_query(self.path.clone())
            .build()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timestamp: DateTime<Utc>,
        scheme: Scheme,
        host: String,
        port: u16,
        path: String,
        method: String,
        version: u8,
        headers: HashMap<String, String>,
        body: bytes::Bytes,
    ) -> Self {
        Self {
            timestamp,
            scheme,
            host,
            port,
            path,
            method,
            version,
            headers,
            body,
        }
    }

    pub fn version_str(&self) -> &str {
        match self.version {
            0 => "1.0",
            1 => "1.1",
            _ => "1.1", // fallback
        }
    }

    pub fn line_pretty(&self) -> String {
        let scheme_str = match self.scheme {
            Scheme::Http => "http",
            Scheme::Https => "https",
        };

        format!(
            "{} {}://{}:{}{} HTTP/{}",
            self.method,
            scheme_str,
            self.host,
            self.port,
            self.path,
            self.version_str()
        )
    }

    pub fn content_type(&self) -> ContentType {
        parse_content_type(&self.headers)
    }

    pub fn request_line(&self) -> String {
        match self.scheme {
            Scheme::Http => {
                format!(
                    "{} {}://{}:{}{} HTTP/{}",
                    self.method,
                    "http",
                    self.host,
                    self.port,
                    self.path,
                    self.version_str()
                )
            }
            Scheme::Https => {
                format!("{} {} HTTP/{}", self.method, self.path, self.version_str())
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.request_line().as_bytes());
        out.extend_from_slice(b"\r\n");

        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");

        headers.remove("Host");
        headers.remove("host");

        headers.insert("Host".to_string(), self.host.to_string());

        headers.insert("Content-Length".to_string(), self.body.len().to_string());

        for (k, v) in &headers {
            out.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
        }

        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(self.body.as_ref());

        out
    }

    pub fn target_host(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone)]
pub struct InterceptedResponse {
    pub timestamp: DateTime<Utc>,
    pub status: u16,
    pub reason: String,
    pub version: u8,
    pub headers: HashMap<String, String>,
    pub body: bytes::Bytes,
}

impl InterceptedResponse {
    pub fn from_http(parts: &http::response::Parts, body_bytes: &bytes::Bytes) -> Self {
        InterceptedResponse {
            timestamp: Utc::now(),
            status: parts.status.as_u16(),
            reason: parts.status.canonical_reason().unwrap_or("").to_string(),
            version: match parts.version {
                http::Version::HTTP_10 => 0,
                http::Version::HTTP_11 => 1,
                http::Version::HTTP_2 => 2,
                http::Version::HTTP_3 => 2,
                _ => 1, // fallback
            },
            headers: parts
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: body_bytes.clone(),
        }
    }

    pub fn request_line(&self) -> String {
        let version_str = match self.version {
            0 => "1.0",
            1 => "1.1",
            _ => "1.1", // default fallback
        };
        format!("HTTP/{} {} {}", version_str, self.status, self.reason)
    }

    pub fn mapped_headers(&self) -> HashMap<String, String> {
        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");
        headers.insert("Content-Length".to_string(), self.body.len().to_string());
        headers
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.request_line().as_bytes());
        out.extend_from_slice(b"\r\n");

        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");
        headers.insert("Content-Length".to_string(), self.body.len().to_string());

        for (k, v) in &headers {
            out.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
        }

        out.extend_from_slice(b"\r\n");
        if !self.body.is_empty() {
            out.extend_from_slice(self.body.as_ref());
            out.extend_from_slice(b"\r\n");
        }

        out
    }

    pub fn content_type(&self) -> ContentType {
        parse_content_type(&self.headers)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Bmp,
    Csv,
    Tsv,
    Gif,
    Html,
    Jpeg,
    Json,
    Md,
    Png,
    Text,
    Toml,
    Unknown,
    Webp,
    XIcon,
    Xml,
    Yaml,
}

pub fn parse_content_type(headers: &HashMap<String, String>) -> ContentType {
    let content_type = headers
        .get("Content-Type")
        .or_else(|| headers.get("content-type"))
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if content_type.contains("json") {
        ContentType::Json
    } else if content_type.contains("bmp") {
        ContentType::Bmp
    } else if content_type.contains("xml") {
        ContentType::Xml
    } else if content_type.contains("csv") {
        ContentType::Csv
    } else if content_type.contains("tab-separated-values") {
        ContentType::Tsv
    } else if content_type.contains("markdown") {
        ContentType::Md
    } else if content_type.contains("html") {
        ContentType::Html
    } else if content_type.contains("toml") {
        ContentType::Toml
    } else if content_type.contains("x-yaml") {
        ContentType::Yaml
    } else if content_type.contains("png") {
        ContentType::Png
    } else if content_type.contains("jpeg") {
        ContentType::Webp
    } else if content_type.contains("gif") {
        ContentType::Gif
    } else if content_type.contains("webp") {
        ContentType::Jpeg
    } else if content_type.contains("x-icon") {
        ContentType::XIcon
    } else if content_type.contains("text") || content_type.starts_with("text/") {
        ContentType::Text
    } else {
        ContentType::Unknown
    }
}
