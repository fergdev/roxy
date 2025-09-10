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

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use http::Version;
    use std::collections::HashSet;
    use std::str::FromStr;

    #[test]
    fn display_formats_match_expected() {
        let cases = &[
            (Version::HTTP_09, "HTTP/0.9"),
            (Version::HTTP_10, "HTTP/1.0"),
            (Version::HTTP_11, "HTTP/1.1"),
            (Version::HTTP_2, "HTTP/2.0"),
            (Version::HTTP_3, "HTTP/3.0"),
        ];
        for (v, expected) in cases {
            let hv = HttpVersion(*v);
            assert_eq!(hv.to_string(), *expected, "display for {:?}", v);
        }
    }

    #[test]
    fn parse_valid_versions() {
        let ok = &[
            ("HTTP/0.9", Version::HTTP_09),
            ("HTTP/1.0", Version::HTTP_10),
            ("HTTP/1.1", Version::HTTP_11),
            ("HTTP/2.0", Version::HTTP_2),
            ("HTTP/3.0", Version::HTTP_3),
        ];
        for (s, v) in ok {
            let hv = HttpVersion::from_str(s).expect("should parse");
            assert_eq!(Version::from(hv), *v, "parsed {:?} incorrectly", s);
        }
    }

    #[test]
    fn parse_h2_h3_aliases() {
        let h2 = HttpVersion::from_str("HTTP/2").expect("HTTP/2 alias");
        let h3 = HttpVersion::from_str("HTTP/3").expect("HTTP/3 alias");
        assert_eq!(Version::from(h2), Version::HTTP_2);
        assert_eq!(Version::from(h3), Version::HTTP_3);
    }

    #[test]
    fn parse_invalid_versions_error() {
        let invalid = &[
            "HTTP/1.2", "HTTP/0.8", "HTTP/2.1", "HTTP/", "1.1", "", "HTTP/1", "HTTP/03",
            "http/1.1", // case sensitive
        ];
        for s in invalid {
            assert!(
                HttpVersion::from_str(s).is_err(),
                "expected parse error for {:?}",
                s
            );
        }
    }

    #[test]
    fn conversions_round_trip() {
        let originals = [
            Version::HTTP_09,
            Version::HTTP_10,
            Version::HTTP_11,
            Version::HTTP_2,
            Version::HTTP_3,
        ];
        for v in originals {
            let hv = HttpVersion::from(v);
            let back: Version = hv.into();
            assert_eq!(back, v, "round-trip conversion failed for {:?}", v);
        }
    }

    #[test]
    fn hashing_and_equality() {
        let mut set: HashSet<HttpVersion> = HashSet::new();
        set.insert(HttpVersion(Version::HTTP_11));
        set.insert(HttpVersion(Version::HTTP_11));
        set.insert(HttpVersion(Version::HTTP_2));
        assert!(set.contains(&HttpVersion(Version::HTTP_11)));
        assert!(set.contains(&HttpVersion(Version::HTTP_2)));
        assert_eq!(set.len(), 2, "duplicates should collapse via Eq/Hash");
    }
}
