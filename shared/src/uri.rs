use std::{fmt::Display, net::SocketAddr, str::FromStr};

use http::{Uri, uri::InvalidUri};
use rustls::pki_types::{InvalidDnsNameError, ServerName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RUri {
    pub inner: Uri,
}

impl Display for RUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.inner))
    }
}

impl RUri {
    pub fn new(uri: Uri) -> Self {
        RUri { inner: uri }
    }

    pub fn and(&self, other: &Uri, scheme: http::uri::Scheme) -> Result<RUri, http::Error> {
        let mut uri = Uri::builder();
        uri = uri.scheme(scheme);

        if let Some(authority) = other.authority().or(self.inner.authority()) {
            uri = uri.authority(authority.clone())
        }

        if let Some(pg) = other.path_and_query().or(self.inner.path_and_query()) {
            uri = uri.path_and_query(pg.clone());
        }

        Ok(RUri::new(uri.build()?))
    }

    pub fn scheme_str(&self) -> Option<&str> {
        self.inner.scheme_str()
    }

    pub fn host(&self) -> &str {
        self.inner.host().unwrap_or("localhost")
    }

    pub fn path(&self) -> &str {
        self.inner.path()
    }

    pub fn path_and_query(&self) -> &str {
        self.inner
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or(self.inner.path())
    }

    pub fn query(&self) -> &str {
        self.inner.query().unwrap_or("")
    }

    pub fn port(&self) -> u16 {
        match self.inner.port_u16() {
            Some(port) => port,
            None => match self.inner.scheme() {
                Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                _ => 80,
            },
        }
    }

    pub fn inner(&self) -> Uri {
        self.inner.clone()
    }

    pub fn valid_authority(&self) -> bool {
        self.inner.authority().is_some()
    }

    pub fn host_port(&self) -> String {
        format!("{}:{}", self.host(), self.port())
    }
    pub fn host_port_scheme(&self) -> String {
        format!("{}://{}:{}", self.scheme(), self.host(), self.port())
    }

    pub fn is_tls(&self) -> bool {
        matches!(self.inner.scheme_str(), Some("https"))
    }

    pub fn scheme(&self) -> Scheme {
        if self.is_tls() {
            Scheme::Https
        } else {
            Scheme::Http
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    pub fn parse(value: &str) -> Option<Scheme> {
        match value {
            "https" => Some(Scheme::Https),
            "http" => Some(Scheme::Http),
            _ => None,
        }
    }
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

impl TryFrom<&RUri> for ServerName<'static> {
    type Error = InvalidDnsNameError;

    fn try_from(value: &RUri) -> Result<Self, Self::Error> {
        let host = value.host().to_string();
        ServerName::try_from(host)
    }
}

impl FromStr for RUri {
    type Err = InvalidUri;

    #[inline]
    fn from_str(s: &str) -> Result<RUri, InvalidUri> {
        let inner = Uri::try_from(s.as_bytes())?;
        Ok(RUri { inner })
    }
}

impl TryInto<Uri> for RUri {
    type Error = InvalidUri;

    fn try_into(self) -> Result<Uri, InvalidUri> {
        Ok(self.inner.clone())
    }
}

impl From<Uri> for RUri {
    fn from(v: Uri) -> RUri {
        RUri::new(v)
    }
}

impl From<&Uri> for RUri {
    fn from(v: &Uri) -> RUri {
        RUri::new(v.clone())
    }
}

impl TryFrom<SocketAddr> for RUri {
    type Error = InvalidUri;
    fn try_from(v: SocketAddr) -> Result<RUri, InvalidUri> {
        format!("{}:{}", v.ip(), v.port()).parse()
    }
}
