use std::net::SocketAddr;

use bytes::Bytes;

use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use http::response::Parts;
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
use roxy_shared::uri::RUri;
use roxy_shared::uri::Scheme;

use http::HeaderMap;
use roxy_shared::body::BytesBody;
use roxy_shared::version::HttpVersion;
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

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
    pub(crate) fn new(
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
    pub body: Bytes,
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
            body: Bytes::new(),
            trailers: None,
        }
    }
}

impl InterceptedRequest {
    pub fn from_http(
        uri: RUri,
        alpn: AlpnProtocol,
        parts: http::request::Parts,
        body_bytes: Bytes,
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
    pub body: Bytes,
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
            body: Bytes::new(),
            trailers: None,
        }
    }
}

impl InterceptedResponse {
    pub fn from_http(parts: Parts, body_bytes: Bytes, trailers: Option<HeaderMap>) -> Self {
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
        format!("{} {}", self.version, self.status)
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
