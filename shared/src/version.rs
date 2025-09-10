use std::{
    fmt::{self, Display},
    str::FromStr,
};

use http::Version;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub struct HttpVersion(pub http::Version);

impl Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.0 {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            Version::HTTP_3 => "HTTP/3.0",
            _ => "UNKNOWN",
        };
        f.write_str(s)
    }
}

impl FromStr for HttpVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = match s {
            "HTTP/0.9" => Version::HTTP_09,
            "HTTP/1.0" => Version::HTTP_10,
            "HTTP/1.1" => Version::HTTP_11,
            "HTTP/2.0" | "HTTP/2" => Version::HTTP_2,
            "HTTP/3.0" | "HTTP/3" => Version::HTTP_3,
            _ => return Err(()),
        };
        Ok(HttpVersion(v))
    }
}

impl From<Version> for HttpVersion {
    fn from(v: Version) -> Self {
        HttpVersion(v)
    }
}

impl From<HttpVersion> for Version {
    fn from(h: HttpVersion) -> Self {
        h.0
    }
}
