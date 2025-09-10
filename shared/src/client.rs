use crate::RoxyCA;
use crate::alpn::AlpnProtocol;
use crate::body::BytesBody;
use crate::http::HttpEmitter;
use crate::http::HttpError;
use crate::http::HttpResponse;
use crate::http::NoOpListener;
use crate::http::connect_proxy;
use crate::http::upstream_h2;
use crate::http::upstream_https;
use crate::http::uptstream_http;
use crate::http::uptstream_http_with_proxy;
use crate::tls::TlsConfig;
use crate::tls::client_tls;
use crate::tls::client_tls_native;
use crate::uri::RUri;
use http::Request;
use http::Version;
use http::uri::Scheme;
use hyper_util::rt::tokio::WithHyperIo;
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tracing::warn;

use crate::h3_client::h3_with_proxy;

#[derive(Debug)]
pub struct RClientBuilder {
    proxy_uri: Option<RUri>,
    roxy_ca: Option<RoxyCA>,
    emitter: Option<Box<dyn HttpEmitter>>,
    alpns: Vec<AlpnProtocol>,
    use_rustls: bool,
    tls_config: Option<TlsConfig>,
}

impl RClientBuilder {
    fn new() -> Self {
        Self {
            proxy_uri: None,
            roxy_ca: None,
            emitter: None,
            use_rustls: true,
            alpns: vec![
                AlpnProtocol::Http1,
                AlpnProtocol::Http2,
                AlpnProtocol::Http3,
            ],
            tls_config: None,
        }
    }

    pub fn use_native_ls(mut self) -> Self {
        self.use_rustls = false;
        self
    }
    pub fn with_proxy(mut self, uri: RUri) -> Self {
        self.proxy_uri = Some(uri);
        self
    }
    pub fn with_roxy_ca(mut self, roxy_ca: RoxyCA) -> Self {
        self.roxy_ca = Some(roxy_ca);
        self
    }
    pub fn with_emitter(mut self, emitter: Box<dyn HttpEmitter>) -> Self {
        self.emitter = Some(emitter);
        self
    }
    pub fn with_alpns(mut self, alpns: Vec<AlpnProtocol>) -> Self {
        self.alpns = alpns;
        self
    }
    pub fn with_tls_config(mut self, tls_config: TlsConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    pub fn build(self) -> ClientContext {
        ClientContext {
            proxy_uri: self.proxy_uri,
            roxy_ca: self.roxy_ca,
            use_rustls: self.use_rustls,
            emitter: self.emitter.unwrap_or(Box::new(NoOpListener {})),
            alpns: self.alpns.iter().map(|f| f.to_bytes().to_vec()).collect(),
            tls_config: self.tls_config.unwrap_or_default(),
        }
    }
}

#[derive(Debug)]
pub struct ClientContext {
    proxy_uri: Option<RUri>,
    use_rustls: bool,
    roxy_ca: Option<RoxyCA>,
    emitter: Box<dyn HttpEmitter>,
    alpns: Vec<Vec<u8>>,
    tls_config: TlsConfig,
}

impl ClientContext {
    pub fn builder() -> RClientBuilder {
        RClientBuilder::new()
    }

    pub async fn request(&self, request: Request<BytesBody>) -> Result<HttpResponse, HttpError> {
        if request.version() == Version::HTTP_3 {
            self.h3_client_call(request).await
        } else if request.uri().scheme() == Some(&Scheme::HTTPS) {
            self.do_tls(request).await
        } else if let Some(proxy_uri) = &self.proxy_uri {
            uptstream_http_with_proxy(proxy_uri, request, self.emitter.as_ref()).await
        } else {
            uptstream_http(request, self.emitter.as_ref()).await
        }
    }

    async fn do_tls(&self, request: Request<BytesBody>) -> Result<HttpResponse, HttpError> {
        let roxy_ca = self.roxy_ca.as_ref().ok_or_else(|| HttpError::Alpn)?;
        let stream = if let Some(proxy_uri) = &self.proxy_uri {
            connect_proxy(proxy_uri, request.uri()).await?
        } else {
            let addr = format!(
                "{}:{}",
                request.uri().host().unwrap_or("localhost"),
                request.uri().port_u16().unwrap_or(443)
            );

            WithHyperIo::new(TcpStream::connect(addr).await?)
        };

        let server_name: ServerName = request
            .uri()
            .host()
            .unwrap_or("localhost")
            .to_string()
            .try_into()?;

        let (stream, alpn) = if self.use_rustls {
            client_tls(
                server_name,
                stream,
                self.alpns.clone(),
                roxy_ca.roots(),
                self.emitter.as_ref(),
                &self.tls_config,
            )
            .await?
        } else {
            let alpns: Vec<String> = self
                .alpns
                .iter()
                .filter_map(|p| String::from_utf8(p.clone()).ok())
                .collect();
            let alpns: Vec<&str> = alpns.iter().map(|p| p.as_ref()).collect();
            client_tls_native(
                server_name,
                stream,
                alpns.as_slice(),
                roxy_ca.clone(),
                self.emitter.as_ref(),
            )
            .await?
        };

        match alpn {
            AlpnProtocol::Http2 => upstream_h2(stream, request, self.emitter.as_ref()).await,
            AlpnProtocol::Http1 => upstream_https(stream, request, self.emitter.as_ref()).await,
            _ => {
                warn!("Unknow alpn negotiated {:?}", alpn);
                upstream_https(stream, request, self.emitter.as_ref()).await
            }
        }
    }
    pub async fn h3_client_call(
        &self,
        request: Request<BytesBody>,
    ) -> Result<HttpResponse, HttpError> {
        let roxy_ca = self.roxy_ca.as_ref().ok_or_else(|| HttpError::Alpn)?;
        h3_with_proxy(
            self.proxy_uri.as_ref(),
            roxy_ca.roots(),
            request,
            self.emitter.as_ref(),
        )
        .await
    }
}
