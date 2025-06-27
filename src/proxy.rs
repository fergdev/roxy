use anyhow::anyhow;
use chrono::Utc;
use once_cell::sync::Lazy;
use rcgen::{CertificateParams, DnType, IsCa};
use rustls::ClientConfig;
use rustls::pki_types::{PrivateKeyDer, ServerName};
use snowflake::SnowflakeIdGenerator;
use tokio::io::BufReader;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::timeout;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use httparse::{EMPTY_HEADER, Request, Response, Status};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::{
    TlsAcceptor, TlsConnector,
    rustls::{RootCertStore, server::ServerConfig},
};
use tracing::{debug, error};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::certs::RoxyCA;
use crate::event::{AppEvent, Event};
use crate::flow::{InterceptedRequest, InterceptedResponse};
use crate::interceptor::{Intercepted, ScriptEngine};

static ID_GENERATOR: Lazy<Mutex<SnowflakeIdGenerator>> = Lazy::new(|| {
    let generator = SnowflakeIdGenerator::new(1, 1);
    Mutex::new(generator)
});

async fn next_id() -> i64 {
    ID_GENERATOR.lock().await.generate()
}

pub fn start_proxy(
    port: u16,
    tx: UnboundedSender<Event>,
    roxy_ca: RoxyCA,
    script_engine: Option<ScriptEngine>,
) -> std::io::Result<()> {
    let host = format!("127.0.0.1:{}", port);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&host).await.unwrap();
        debug!("Proxy listening on {}", host);

        let ca = Arc::new(roxy_ca);
        loop {
            let (socket, addr) = listener.accept().await.unwrap();
            debug!("Accepted connection from {}", addr);
            let tx_clone = tx.clone();
            if let Err(e) =
                handle_connection(socket, tx_clone, ca.clone(), script_engine.clone()).await
            {
                error!("Error in connection: {:?}", e);
            };
        }
    });
    Ok(())
}

pub async fn handle_connection(
    stream: TcpStream,
    tx: UnboundedSender<Event>,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut prebuffer = Vec::new();
    let mut buf = [0u8; 1024];

    let mut total_read = 0;
    let max_header_size = 8192;
    let timeout_duration = Duration::from_secs(5);

    debug!("Reading initial buffer...");

    // Read until we hit end-of-headers or timeout
    loop {
        if prebuffer.windows(4).any(|w| w == b"\r\n\r\n") {
            debug!("Found end of headers in prebuffer");
            break;
        }

        let n = match timeout(timeout_duration, reader.read(&mut buf)).await {
            Ok(Ok(0)) => break, // connection closed
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow!("Timeout waiting for request headers")),
        };

        prebuffer.extend_from_slice(&buf[..n]);
        total_read += n;

        if total_read > max_header_size {
            return Err(anyhow::anyhow!("Header too large"));
        }
    }

    debug!(
        "Initial request buffer:\n{}",
        String::from_utf8_lossy(&prebuffer)
    );

    let is_https = prebuffer.starts_with(b"CONNECT ");
    let stream = reader.into_inner();
    let reader = BufReader::new(stream);

    if is_https {
        let line = String::from_utf8_lossy(&prebuffer);
        let host = line
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown:443")
            .split(':')
            .next()
            .unwrap_or("unknown")
            .to_string();

        debug!("Routing to HTTPS handler for host: {}", host);
        handle_https_connect(reader, &host, tx, ca, script_engine).await?;
    } else {
        debug!("Routing to HTTP handler");
        handle_http_request(reader, prebuffer, tx, script_engine).await?;
    }

    Ok(())
}

pub async fn handle_http_request(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    mut prebuffer: Vec<u8>,
    tx: UnboundedSender<Event>,
    script_engine: Option<ScriptEngine>,
) -> anyhow::Result<()> {
    debug!("HTTP prebuffered:\n{}", String::from_utf8_lossy(&prebuffer));
    let id = next_id().await;

    // Read full headers
    let mut buf = [0u8; 1024];
    while !prebuffer.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        prebuffer.extend_from_slice(&buf[..n]);
    }

    // Parse request
    let mut headers = [EMPTY_HEADER; 32];
    let mut req = Request::new(&mut headers);
    let status = req.parse(&prebuffer)?;
    let header_end = match status {
        Status::Complete(pos) => pos,
        Status::Partial => return Err(anyhow::anyhow!("Incomplete HTTP headers")),
    };

    let method = req.method.unwrap_or("GET");
    let path = req.path.unwrap_or("/");
    let version = req.version.unwrap_or(1);
    let request_line = format!("{method} {path} HTTP/1.{version}");

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

    let content_length = headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
        .and_then(|h| std::str::from_utf8(h.value).ok()?.parse::<usize>().ok())
        .unwrap_or(0);

    // Read body if needed
    let mut body = prebuffer[header_end..].to_vec();
    while body.len() < content_length {
        let mut temp = vec![0u8; content_length - body.len()];
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    tx.send(Event::App(AppEvent::Request(InterceptedRequest {
        id,
        method: method.to_string(),
        url: path.to_string(),
        version,
        timestamp: Utc::now(),
        headers: header_map.clone(),
        body: String::from_utf8(body.clone()).ok(),
    })))?;

    let host = header_map
        .get("Host")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let addr = format!("{host}:80");
    let mut server = TcpStream::connect(&addr).await?;

    // Forward full request to server
    let mut request_bytes = Vec::new();
    request_bytes.extend_from_slice(request_line.as_bytes());
    request_bytes.extend_from_slice(b"\r\n");
    for (k, v) in &header_map {
        request_bytes.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    request_bytes.extend_from_slice(b"\r\n");
    request_bytes.extend_from_slice(&body);

    let mut req = Intercepted {
        url: "http://example.com/".into(),
        headers: HashMap::new(),
        body: Some("hello".into()),
    };

    script_engine
        .as_ref()
        .map(|engine| engine.intercept_request(&mut req));

    server.write_all(&request_bytes).await?;

    handle_response(server, stream, tx, id, script_engine).await?;

    Ok(())
}

async fn handle_response(
    mut server: TcpStream,
    mut stream: impl AsyncWrite + Unpin,
    tx: UnboundedSender<Event>,
    id: i64,
    script_engine: Option<ScriptEngine>,
) -> anyhow::Result<()> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];

    // Read until we get full headers
    let mut headers_end = None;
    while headers_end.is_none() {
        let n = server.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);

        headers_end = buffer
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|pos| pos + 4);

        if buffer.len() > 8192 {
            return Err(anyhow::anyhow!("Response headers too large"));
        }
    }

    let headers_end =
        headers_end.ok_or_else(|| anyhow::anyhow!("Failed to parse response headers"))?;
    let header_buf = &buffer[..headers_end];

    let mut headers = [EMPTY_HEADER; 32];
    let mut res = Response::new(&mut headers);
    res.parse(header_buf)?;

    let status = res.code.unwrap_or(0);
    let version = res.version.unwrap_or(1);
    let reason = res.reason.unwrap_or("");
    let status_line = format!("HTTP/1.{} {} {}", version, status, reason);

    let mut header_map = HashMap::new();
    for h in headers.iter().filter(|h| !h.name.is_empty()) {
        header_map.insert(
            h.name.to_string(),
            String::from_utf8_lossy(h.value).to_string(),
        );
    }

    let content_length = header_map
        .get("Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    // Read body if needed
    let mut body = buffer[headers_end..].to_vec();
    while body.len() < content_length {
        let n = server.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    script_engine.as_ref().map(|engine| {
        engine.intercept_response(&mut Intercepted {
            url: "http://example.com/".into(),
            headers: header_map.clone(),
            body: Some(String::from_utf8_lossy(&body).to_string()),
        })
    });

    // Emit intercepted response
    tx.send(Event::App(AppEvent::Response(InterceptedResponse {
        id,
        timestamp: Utc::now(),
        status,
        version,
        reason: reason.to_string(),
        headers: header_map.clone(),
        body: String::from_utf8(body.clone()).ok(),
    })))?;

    // Rebuild and send response to client
    let mut response_bytes = Vec::new();
    response_bytes.extend_from_slice(status_line.as_bytes());
    response_bytes.extend_from_slice(b"\r\n");
    for (k, v) in &header_map {
        response_bytes.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    response_bytes.extend_from_slice(b"\r\n");
    response_bytes.extend_from_slice(&body);
    stream.write_all(&response_bytes).await?;

    Ok(())
}

pub async fn handle_https_connect(
    reader: BufReader<TcpStream>,
    target_host: &str,
    tx: UnboundedSender<Event>,
    ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
) -> anyhow::Result<()> {
    let id = next_id().await;

    // TODO: fillout properly
    tx.send(Event::App(AppEvent::Request(InterceptedRequest {
        id,
        timestamp: Utc::now(),
        method: "CONNECT".into(),
        url: format!("{target_host}:443"),
        version: 1,
        headers: HashMap::from([("Host".into(), target_host.into())]),
        body: None,
    })))?;

    let mut client = reader.into_inner();
    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    let mut params = CertificateParams::new(vec![target_host.to_string()])?;
    params
        .distinguished_name
        .push(DnType::CommonName, target_host);
    params.is_ca = IsCa::NoCa;
    let leaf = params.self_signed(&ca.key_pair)?;
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::try_from(ca.key_pair.serialize_der()).unwrap(),
        )?;

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let client_tls = acceptor.accept(client).await?;

    let server = TcpStream::connect((target_host, 443)).await?;
    let mut root_store = RootCertStore::empty();
    root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
    let tls_client_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(tls_client_config));
    let server_tls = connector
        .connect(ServerName::try_from(target_host.to_string())?, server)
        .await?;

    let (cr, cw) = tokio::io::split(client_tls);
    let (sr, sw) = tokio::io::split(server_tls);

    tokio::spawn(copy_and_inspect(
        cr,
        sw,
        tx.clone(),
        Direction::Request,
        id,
        script_engine.clone(),
    ));
    tokio::spawn(copy_and_inspect(
        sr,
        cw,
        tx,
        Direction::Response,
        id,
        script_engine,
    ));

    Ok(())
}

async fn copy_and_inspect<R, W>(
    mut reader: R,
    mut writer: W,
    tx: UnboundedSender<Event>,
    direction: Direction,
    id: i64,
    script_engine: Option<ScriptEngine>,
) where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = [0u8; 8192];
    let mut collected = Vec::new();
    let mut emitted = false;

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                let chunk = &buf[..n];
                if writer.write_all(chunk).await.is_err() {
                    break;
                }

                if !emitted {
                    collected.extend_from_slice(chunk);
                    if let Some(parsed) = parse_http_message(&collected, direction) {
                        match parsed {
                            ParsedHttp::Request {
                                method,
                                path,
                                version,
                                headers,
                                body,
                            } => {
                                let inter = InterceptedRequest {
                                    id,
                                    timestamp: Utc::now(),
                                    method,
                                    url: path,
                                    version,
                                    headers,
                                    body,
                                };
                                let mut intercepted = Intercepted {
                                    url: inter.url.clone(),
                                    headers: inter.headers.clone(),
                                    body: inter.body.clone(),
                                };

                                if let Some(engine) = script_engine.as_ref() {
                                    engine.intercept_response(&mut intercepted).unwrap();
                                };
                                tx.send(Event::App(AppEvent::Request(inter))).ok();
                            }
                            ParsedHttp::Response {
                                version,
                                code,
                                reason,
                                headers,
                                body,
                            } => {
                                let inter = InterceptedResponse {
                                    id,
                                    timestamp: Utc::now(),
                                    version,
                                    status: code,
                                    reason,
                                    headers,
                                    body,
                                };
                                let mut intercepted = Intercepted {
                                    url: "this is crap".to_string(),
                                    headers: inter.headers.clone(),
                                    body: inter.body.clone(),
                                };
                                if let Some(engine) = script_engine.as_ref() {
                                    engine.intercept_response(&mut intercepted).unwrap();
                                };
                                tx.send(Event::App(AppEvent::Response(inter))).ok();
                            }
                        }
                        emitted = true;
                    } else if collected.len() > 16384 {
                        // avoid collecting infinite buffer if parsing fails
                        emitted = true;
                    }
                }
            }
            Err(_) => break,
        }
    }
}

#[derive(Debug)]
pub enum ParsedHttp {
    Request {
        method: String,
        path: String,
        version: u8,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
    Response {
        version: u8,
        code: u16,
        reason: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    Request,
    Response,
}

fn parse_http_message(buf: &[u8], direction: Direction) -> Option<ParsedHttp> {
    let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let header_bytes = &buf[..header_end];
    let body_bytes = &buf[header_end..];

    match direction {
        Direction::Request => {
            let mut headers = [EMPTY_HEADER; 32];
            let mut req = Request::new(&mut headers);
            let _ = req.parse(header_bytes).ok()?;

            // Extract data early before using headers directly
            let method = req.method.unwrap_or("GET").to_string();
            let path = req.path.unwrap_or("/").to_string();
            let version = req.version.unwrap_or(1);

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

            Some(ParsedHttp::Request {
                method,
                path,
                version,
                headers: header_map.clone(),
                body: std::str::from_utf8(body_bytes).ok().map(|s| s.to_string()),
            })
        }

        Direction::Response => {
            let mut headers = [EMPTY_HEADER; 32];
            let mut resp = Response::new(&mut headers);
            let _ = resp.parse(header_bytes).ok()?;

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

            Some(ParsedHttp::Response {
                version,
                code,
                reason,
                headers: header_map,
                body: std::str::from_utf8(body_bytes).ok().map(|s| s.to_string()),
            })
        }
    }
}
