use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use httparse::{EMPTY_HEADER, Request, Response};

use rustls::pki_types::CertificateDer;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{RwLock, watch},
};

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

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.notifier.subscribe()
    }
}

impl Default for FlowStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Flow {
    pub id: i64,
    pub request: Option<InterceptedRequest>,
    pub response: Option<InterceptedResponse>,
    pub error: Option<String>,
    pub cert_info: Option<Vec<CertInfo>>,
}

impl Flow {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            request: None,
            response: None,
            error: None,
            cert_info: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CertInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
}

impl CertInfo {
    pub fn from_der(cert: &CertificateDer<'_>) -> Option<Self> {
        use x509_parser::parse_x509_certificate;

        let (_, parsed) = parse_x509_certificate(cert.as_ref()).ok()?;
        Some(CertInfo {
            subject: parsed.subject().to_string(),
            issuer: parsed.issuer().to_string(),
            not_before: parsed.validity().not_before.to_string(),
            not_after: parsed.validity().not_after.to_string(),
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
    pub body: Option<String>,
}

impl InterceptedRequest {
    pub fn new(
        timestamp: DateTime<Utc>,
        scheme: Scheme,
        host: String,
        port: u16,
        path: String,
        method: String,
        version: u8,
        headers: HashMap<String, String>,
        body: Option<String>,
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
                // After CONNECT/TLS, use origin-form
                format!("{} {} HTTP/{}", self.method, self.path, self.version_str())
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.request_line().as_bytes());
        out.extend_from_slice(b"\r\n");

        // Clone headers so we can modify them safely
        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");

        headers.remove("Host");
        headers.remove("host");

        headers.insert("Host".to_string(), self.host.to_string());

        // Set or update the Content-Length if body is present
        if let Some(body) = &self.body {
            headers.insert("Content-Length".to_string(), body.len().to_string());
        }

        for (k, v) in &headers {
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }

        out.extend_from_slice(b"\r\n");

        if let Some(body) = &self.body {
            out.extend_from_slice(body.as_bytes());
        }

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
    pub body: Option<String>,
}

impl InterceptedResponse {
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

        // Create a local header map so we can insert/update Content-Length
        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");
        if let Some(body) = &self.body {
            headers.insert("Content-Length".to_string(), body.len().to_string());
        }

        for (k, v) in &headers {
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }

        out.extend_from_slice(b"\r\n");

        if let Some(body) = &self.body {
            out.extend_from_slice(body.as_bytes());
        }

        out
    }
}

pub async fn read_http_request<R: AsyncRead + Unpin>(
    reader: &mut R,
    host: String,
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
            let body = Some(String::from_utf8_lossy(&data[header_len..]).to_string());
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
        host,
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
) -> anyhow::Result<InterceptedResponse> {
    let mut buf = vec![0; 65536];
    let n = reader.read(&mut buf).await?;
    let data = &buf[..n];

    let mut headers = [EMPTY_HEADER; 64];
    let mut resp = Response::new(&mut headers);

    // Parse first!
    let status = resp.parse(data)?;

    let response = match status {
        httparse::Status::Complete(header_len) => {
            let version = resp.version.unwrap_or(1);
            let code = resp.code.unwrap_or(200);
            let reason = resp.reason.unwrap_or("").to_string();

            let header_map = headers
                .iter()
                .filter(|h| !h.name.is_empty())
                .map(|h| {
                    (
                        h.name.to_string(),
                        String::from_utf8_lossy(h.value).to_string(),
                    )
                })
                .collect::<HashMap<_, _>>();

            let body = &data[header_len..];
            Ok(InterceptedResponse {
                timestamp: Utc::now(),
                version,
                status: code,
                reason,
                headers: header_map,
                body: Some(String::from_utf8_lossy(body).to_string()),
            })
        }
        httparse::Status::Partial => Err(anyhow::anyhow!("incomplete HTTP response")),
    }?; // <- unwrap Result

    Ok(response)
}
