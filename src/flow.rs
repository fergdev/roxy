use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use httparse::{EMPTY_HEADER, Request, Response};

use rustls::pki_types::CertificateDer;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{RwLock, watch},
};
use tokio_tungstenite::tungstenite::Message;
use tracing::error;

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
    pub async fn new_flow(&self, id: i64) -> Arc<RwLock<Flow>> {
        let flow = Arc::new(RwLock::new(Flow::new(id)));
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
        self.notifier.send(()).unwrap_or_else(|_| {
            error!("Failed to notify subscribers, channel closed");
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
    pub kind: FlowKind,
}

pub enum FlowKind {
    Http(HttpFlow),
    Https(HttpsFlow),
    Ws(WsFlow),
    Wss(WssFlow),
    Http2(Http2Flow),
    // Http3(Http3Flow),
    //TlsOnly(TlsMetadata),
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
    pub handshake: InterceptedRequest,
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub tls_metadata: Option<TlsMetadata>,
    pub cert_info: Option<Vec<CertInfo>>,
}

impl HttpsFlow {
    pub fn new(handshake: InterceptedRequest, request: InterceptedRequest) -> Self {
        Self {
            handshake,
            request,
            response: None,
            tls_metadata: None,
            cert_info: None,
        }
    }
}

pub struct WsFlow {
    pub handshake: InterceptedRequest,
    pub messages: Vec<WsMessage>,
}

impl WsFlow {
    pub fn new(handshake: InterceptedRequest) -> Self {
        Self {
            handshake,
            messages: Vec::new(),
        }
    }
}

pub struct WssFlow {
    pub handshake: InterceptedRequest,
    pub messages: Vec<WsMessage>,
    pub cert_info: Option<Vec<CertInfo>>,
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
    pub fn new(handshake: InterceptedRequest) -> Self {
        Self {
            handshake,
            messages: Vec::new(),
            cert_info: None,
            tls_metadata: None,
        }
    }
}

pub struct Http2Flow {
    pub handshake: InterceptedRequest,
    pub requests: Vec<InterceptedRequest>,
    pub responses: Vec<InterceptedResponse>,
    pub cert_info: Option<Vec<CertInfo>>,
    pub tls_metadata: Option<TlsMetadata>,
}

impl Http2Flow {
    pub fn new(handshake: InterceptedRequest) -> Self {
        Self {
            handshake,
            requests: vec![],
            responses: vec![],
            cert_info: None,
            tls_metadata: None,
        }
    }
}

pub struct TlsMetadata {
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub negotiated_cipher: Option<String>,
    pub peer_cert: Option<CertInfo>,
}

impl Flow {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            timing: Timing::default(),
            kind: FlowKind::Unknown,
        }
    }
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

#[derive(Clone, Debug)]
pub struct CertInfo {
    pub version: u32,
    pub serial: Vec<u8>,
    pub signature_oid: String,
    pub issuer: String,
    pub subject: String,
    pub not_before: String,
    pub not_after: String,
    pub public_key: Vec<u8>,
    pub signature_value: Vec<u8>,
}

impl CertInfo {
    pub fn from_der(cert: &CertificateDer<'_>) -> Option<Self> {
        use x509_parser::parse_x509_certificate;

        let (_, cert) = parse_x509_certificate(cert.as_ref()).ok()?;
        let tbs = &cert.tbs_certificate;
        Some(Self {
            version: tbs.version.0,
            serial: tbs.serial.to_bytes_be(),
            signature_oid: tbs.signature.algorithm.to_id_string(),
            issuer: tbs.issuer.to_string(),
            subject: tbs.subject.to_string(),
            not_before: tbs.validity.not_before.to_datetime().to_string(),
            not_after: tbs.validity.not_after.to_datetime().to_string(),
            public_key: tbs.subject_pki.subject_public_key.data.to_vec(),
            signature_value: cert.signature_value.data.to_vec(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Scheme {
    Http,
    Https,
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
    pub fn from_http(parts: &http::request::Parts, body_bytes: &bytes::Bytes) -> Self {
        InterceptedRequest {
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
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
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
                _ => 3, // fallback
            },
            headers: parts
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: body_bytes.clone().into(),
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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.request_line().as_bytes());
        out.extend_from_slice(b"\r\n");

        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");
        headers.insert("Content-Length".to_string(), self.body.len().to_string());

        for (k, v) in &headers {
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }

        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(self.body.as_ref());

        out
    }

    pub fn content_type(&self) -> ContentType {
        parse_content_type(&self.headers)
    }
}

pub async fn read_http_request<R: AsyncRead + Unpin>(
    reader: &mut R,
    host: &str,
    port: u16,
    scheme: Scheme,
) -> Result<InterceptedRequest> {
    let mut buf = vec![0; 65536];
    let n = reader.read(&mut buf).await?;
    let data = &buf[..n];

    let mut headers = [EMPTY_HEADER; 64];
    let mut req = Request::new(&mut headers);

    let status = req.parse(data)?;
    let (method, path, version, body) = match status {
        httparse::Status::Complete(header_len) => {
            let method = req.method.unwrap_or("GET").to_string();
            let path = req.path.unwrap_or("/").to_string();
            let version = req.version.unwrap_or(1);
            let body = Bytes::copy_from_slice(&data[header_len..]);
            (method, path, version, body)
        }
        httparse::Status::Partial => {
            return Err(anyhow::anyhow!("incomplete HTTP request"));
        }
    };

    let headers_map = headers
        .iter()
        .filter(|h| !h.name.is_empty())
        .map(|h| {
            (
                h.name.to_string(),
                String::from_utf8_lossy(h.value).to_string(),
            )
        })
        .collect();

    Ok(InterceptedRequest::new(
        Utc::now(),
        scheme,
        host.to_string(),
        port,
        path,
        method,
        version,
        headers_map,
        body,
    ))
}

pub async fn read_http_response<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<InterceptedResponse> {
    let mut buf = vec![0; 65536];
    let mut headers = [EMPTY_HEADER; 64];
    let mut resp = Response::new(&mut headers);

    let n = reader.read(&mut buf).await?;
    let status = resp.parse(&buf[..n])?;

    let header_len = match status {
        httparse::Status::Complete(len) => len,
        httparse::Status::Partial => return Err(anyhow::anyhow!("incomplete HTTP response")),
    };

    let version = resp.version.unwrap_or(1);
    let code = resp.code.unwrap_or(200);
    let reason = resp.reason.unwrap_or("").to_string();

    let header_map: HashMap<String, String> = headers
        .iter()
        .filter(|h| !h.name.is_empty())
        .map(|h| {
            (
                h.name.to_string(),
                String::from_utf8_lossy(h.value).to_string(),
            )
        })
        .collect();

    let content_length = header_map
        .get("Content-Length")
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = buf[header_len..n].to_vec(); // already-read body chunk
    while body.len() < content_length {
        let mut chunk = vec![0; content_length - body.len()];
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    Ok(InterceptedResponse {
        timestamp: Utc::now(),
        version,
        status: code,
        reason,
        headers: header_map,
        body: body.into(),
    })
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
        .or_else(|| headers.get("content-type")) // case-insensitive fallback
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
