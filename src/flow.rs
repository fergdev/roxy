use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use httparse::{EMPTY_HEADER, Request, Response};

use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, Clone)]
pub struct Flow {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InterceptedRequest {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub uri: String,
    pub version: u8,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl InterceptedRequest {
    pub fn new(
        id: i64,
        timestamp: DateTime<Utc>,
        method: String,
        uri: String,
        version: u8,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> Self {
        Self {
            id,
            timestamp,
            method,
            uri,
            version,
            headers,
            body,
        }
    }

    pub fn request_line(&self) -> String {
        let version_str = match self.version {
            0 => "1.0",
            1 => "1.1",
            _ => "1.1", // default fallback
        };
        format!("{} {} HTTP/{}", self.method, self.uri, version_str)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.request_line().as_bytes());
        out.extend_from_slice(b"\r\n");

        // Clone headers so we can modify them safely
        let mut headers = self.headers.clone();

        headers.remove("Content-Length");
        headers.remove("content-length");
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
}

#[derive(Debug, Clone)]
pub struct InterceptedResponse {
    pub id: i64,
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

pub async fn read_http_request2<R: AsyncRead + Unpin>(
    reader: &mut R,
    id: i64,
) -> Result<InterceptedRequest> {
    let mut buf = vec![0; 65536];
    let n = reader.read(&mut buf).await?;
    let data = &buf[..n];

    let mut headers = [EMPTY_HEADER; 64];
    let mut req = Request::new(&mut headers);

    let method = req.method.unwrap_or("GET").to_string();
    let path = req.path.unwrap_or("/").to_string();
    let version = req.version.unwrap_or(1);

    let status = req.parse(data)?;
    let body = match status {
        httparse::Status::Complete(header_len) => {
            let body = &data[header_len..];
            Some(String::from_utf8_lossy(body).to_string())
        }
        _ => None,
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
        id,
        Utc::now(),
        method,
        path,
        version,
        headers_map,
        body,
    ))
}

pub async fn read_http_request<R: AsyncRead + Unpin>(
    reader: &mut R,
    id: i64,
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
        id,
        Utc::now(),
        method,
        path,
        version,
        headers_map,
        body,
    ))
}

pub async fn read_http_response2<R: AsyncRead + Unpin>(
    reader: &mut R,
    id: i64,
) -> Result<InterceptedResponse> {
    let mut buf = vec![0; 65536];
    let n = reader.read(&mut buf).await?;
    let data = &buf[..n];

    let mut headers = [EMPTY_HEADER; 64];
    let mut resp = Response::new(&mut headers);

    let version = resp.version.unwrap_or(1);
    let code = resp.code.unwrap_or(200);
    let reason = resp.reason.unwrap_or("").to_string();

    let status = resp.parse(data)?;
    let body = match status {
        httparse::Status::Complete(header_len) => {
            let body = &data[header_len..];
            Some(String::from_utf8_lossy(body).to_string())
        }
        _ => None,
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

    Ok(InterceptedResponse {
        id,
        timestamp: Utc::now(),
        version,
        status: code,
        reason,
        headers: headers_map,
        body,
    })
}
pub async fn read_http_response<R: AsyncRead + Unpin>(
    reader: &mut R,
    id: i64,
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
                id,
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
