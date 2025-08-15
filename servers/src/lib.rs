#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{collections::HashSet, error::Error, fmt::Display, ops::Deref, path::PathBuf, sync::Arc};

use http::Version;
use roxy_shared::{
    RoxyCA,
    alpn::AlpnProtocol,
    content::{ContentType, content_type_ext},
    tls::{RustlsServerConfig, TlsConfig},
    uri::RUri,
};
use rustls::{ServerConfig, pki_types::PrivateKeyDer, sign::CertifiedKey};
use strum::{EnumIter, IntoEnumIterator};
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tracing::debug;

use crate::{
    h1::{h1_server, h1s_server},
    h2::h2_server,
    h3::h3_server,
};

pub mod h1;
pub mod h2;
pub mod h3;
pub mod serve;
pub mod web_transport;
pub mod ws;

pub static H09_BODY: &str = "H09";
pub static H10_BODY: &str = "H10";
pub static H11_BODY: &str = "H11";

pub static H09S_BODY: &str = "H09S";
pub static H10S_BODY: &str = "H10S";
pub static H11S_BODY: &str = "H11S";

pub static H2_BODY: &str = "H2";
pub static H3_BODY: &str = "H3";

#[derive(EnumIter, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum HttpServers {
    H09,
    H10,
    H11,
    H09S,
    H10S,
    H11S,
    H2,
    H3,
}

impl Display for HttpServers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{self:?}, {}, {:?}, {:?}",
            self.scheme(),
            self.alpn(),
            self.version()
        ))
    }
}

impl HttpServers {
    pub fn alpn(&self) -> AlpnProtocol {
        match self {
            HttpServers::H09 | HttpServers::H10 | HttpServers::H11 => AlpnProtocol::None,
            HttpServers::H09S | HttpServers::H10S | HttpServers::H11S => AlpnProtocol::Http1,
            HttpServers::H2 => AlpnProtocol::Http2,
            HttpServers::H3 => AlpnProtocol::Http3,
        }
    }
    pub fn version(&self) -> Version {
        match self {
            HttpServers::H09 => Version::HTTP_09,
            HttpServers::H10 => Version::HTTP_10,
            HttpServers::H11 => Version::HTTP_11,
            HttpServers::H09S => Version::HTTP_09,
            HttpServers::H10S => Version::HTTP_10,
            HttpServers::H11S => Version::HTTP_11,
            HttpServers::H2 => Version::HTTP_2,
            HttpServers::H3 => Version::HTTP_3,
        }
    }
    pub fn marker(&self) -> &str {
        match self {
            HttpServers::H09 => H09_BODY,
            HttpServers::H10 => H10_BODY,
            HttpServers::H11 => H11_BODY,
            HttpServers::H09S => H09_BODY,
            HttpServers::H10S => H10_BODY,
            HttpServers::H11S => H11_BODY,
            HttpServers::H2 => H2_BODY,
            HttpServers::H3 => H3_BODY,
        }
    }

    fn scheme(&self) -> &str {
        if matches!(self, HttpServers::H09 | HttpServers::H10 | HttpServers::H11) {
            "http"
        } else {
            "https"
        }
    }

    pub fn is_tls(&self) -> bool {
        !matches!(self, HttpServers::H09 | HttpServers::H10 | HttpServers::H11)
    }

    pub async fn start(
        &self,
        roxy_ca: &RoxyCA,
        tls_config: &TlsConfig,
    ) -> Result<ServerCxt, Box<dyn Error>> {
        let (addr, handle) = match self {
            HttpServers::H09 => h1_server(*self).await?,
            HttpServers::H10 => h1_server(*self).await?,
            HttpServers::H11 => h1_server(*self).await?,
            HttpServers::H09S => h1s_server(*self, roxy_ca, tls_config).await?,
            HttpServers::H10S => h1s_server(*self, roxy_ca, tls_config).await?,
            HttpServers::H11S => h1s_server(*self, roxy_ca, tls_config).await?,
            HttpServers::H2 => h2_server(*self, roxy_ca, tls_config).await?,
            HttpServers::H3 => h3_server(*self, roxy_ca, tls_config).await?,
        };

        let target: RUri = format!("{}://{}:{}", self.scheme(), addr.ip(), addr.port()).parse()?;

        Ok(ServerCxt {
            tls_config: TlsConfig::default(),
            server: *self,
            target,
            handle,
        })
    }

    pub fn set_all() -> HashSet<HttpServers> {
        HttpServers::iter()
            .filter(|s| s != &HttpServers::H09 && s != &HttpServers::H09S)
            .collect()
    }
    pub fn set_not_supported() -> HashSet<HttpServers> {
        let mut set = HashSet::new();
        set.insert(HttpServers::H09);
        set.insert(HttpServers::H09S);
        set
    }

    pub async fn start_all(
        cxt: &RoxyCA,
        tls_config: &TlsConfig,
    ) -> Result<Vec<ServerCxt>, Box<dyn Error>> {
        HttpServers::start_set(HttpServers::set_all(), cxt, tls_config).await
    }

    pub async fn start_set(
        set: HashSet<HttpServers>,
        cxt: &RoxyCA,
        tls_config: &TlsConfig,
    ) -> Result<Vec<ServerCxt>, Box<dyn Error>> {
        let mut cxts = Vec::new();
        for server in set {
            debug!("starting server {server}");
            cxts.push(server.start(cxt, tls_config).await?);
        }
        Ok(cxts)
    }
}

#[derive(Debug)]
pub struct ServerCxt {
    pub tls_config: TlsConfig,
    pub server: HttpServers,
    pub target: RUri,
    pub handle: JoinHandle<()>,
}

impl Drop for ServerCxt {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub fn local_tls_config(
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
    alpns: Vec<Vec<u8>>,
) -> Result<ServerConfig, Box<dyn Error>> {
    let (leaf, key_pair) = roxy_ca.sign_leaf_mult(
        "localhost",
        vec!["localhost".to_string(), "127.0.0.1".to_string()],
    )?;
    let pk_der = PrivateKeyDer::try_from(key_pair.serialize_der())?;
    let provider = tls_config.crypto_provider();
    let certified_key = CertifiedKey::from_der(vec![leaf.der().clone()], pk_der, provider.deref())?;

    let RustlsServerConfig {
        resolver: _,
        mut server_config,
    } = tls_config.rustls_server_config(certified_key)?;

    server_config.alpn_protocols = alpns;
    Ok(server_config)
}

pub fn local_tls_acceptor(
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
    alpns: Vec<Vec<u8>>,
) -> Result<TlsAcceptor, Box<dyn Error>> {
    Ok(TlsAcceptor::from(Arc::new(local_tls_config(
        roxy_ca, tls_config, alpns,
    )?)))
}

pub async fn load_asset(content_type: &ContentType) -> Result<Vec<u8>, std::io::Error> {
    let ext = content_type_ext(content_type);
    let root = env!("CARGO_MANIFEST_DIR"); // TODO: not load this each time
    let file = PathBuf::from(format!("{root}/assets/test.{ext}"));
    debug!("Loading {file:?}");
    tokio::fs::read(&file).await
}
