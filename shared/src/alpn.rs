use bytes::Bytes;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AlpnProtocol {
    Http1,
    Http2,
    Http3,
    Unknown(Bytes),
    #[default]
    None,
}

const ALPN_H3: &[u8] = b"h3";
const ALPN_H3_32: &[u8] = b"h3-32";
const ALPN_H3_31: &[u8] = b"h3-31";
const ALPN_H3_30: &[u8] = b"h3-30";
const ALPN_H3_29: &[u8] = b"h3-29";

const ALPN_H2: &[u8] = b"h2";
const ALPN_H11: &[u8] = b"http/1.1";

impl AlpnProtocol {
    pub fn to_bytes(&self) -> &[u8] {
        match self {
            AlpnProtocol::Http1 => ALPN_H11,
            AlpnProtocol::Http2 => ALPN_H2,
            AlpnProtocol::Http3 => ALPN_H3,
            AlpnProtocol::Unknown(bytes) => bytes,
            AlpnProtocol::None => &[],
        }
    }

    pub fn from_bytes_opt(alpn: Option<&[u8]>) -> Self {
        match alpn {
            Some(bytes) => AlpnProtocol::from_bytes(bytes),
            None => AlpnProtocol::None,
        }
    }

    pub fn from_bytes(alpn: &[u8]) -> Self {
        match alpn {
            ALPN_H3 => AlpnProtocol::Http3,
            ALPN_H2 => AlpnProtocol::Http2,
            ALPN_H11 => AlpnProtocol::Http1,
            _ => AlpnProtocol::Unknown(Bytes::from(alpn.to_owned())),
        }
    }

    pub fn is_tls(&self) -> bool {
        !matches!(self, AlpnProtocol::None | AlpnProtocol::Unknown(_))
    }
}

pub fn alp_h2_h1() -> Vec<Vec<u8>> {
    vec![ALPN_H2.to_vec(), ALPN_H11.to_vec()]
}
pub fn alp_h1_h2() -> Vec<Vec<u8>> {
    vec![ALPN_H11.to_vec(), ALPN_H2.to_vec()]
}
pub fn alp_h1() -> Vec<Vec<u8>> {
    vec![ALPN_H11.to_vec()]
}
pub fn alp_h2() -> Vec<Vec<u8>> {
    vec![ALPN_H2.to_vec()]
}
pub fn alp_h3() -> Vec<Vec<u8>> {
    vec![ALPN_H3.into()]
}
pub fn alp_h3_all() -> Vec<Vec<u8>> {
    vec![
        ALPN_H3.into(),
        ALPN_H3_32.into(),
        ALPN_H3_31.into(),
        ALPN_H3_30.into(),
        ALPN_H3_29.into(),
    ]
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_recognizes_known_protocols() {
        assert_eq!(AlpnProtocol::from_bytes(b"http/1.1"), AlpnProtocol::Http1);
        assert_eq!(AlpnProtocol::from_bytes(b"h2"), AlpnProtocol::Http2);
        assert_eq!(AlpnProtocol::from_bytes(b"h3"), AlpnProtocol::Http3);
    }

    #[test]
    fn from_bytes_opt_handles_none() {
        assert_eq!(AlpnProtocol::from_bytes_opt(None), AlpnProtocol::None);
        assert_eq!(
            AlpnProtocol::from_bytes_opt(Some(b"h2")),
            AlpnProtocol::Http2
        );
    }

    #[test]
    fn unknown_protocol_is_preserved() {
        let raw = b"h3-29";
        let p = AlpnProtocol::from_bytes(raw);
        match &p {
            AlpnProtocol::Unknown(b) => assert_eq!(b.as_ref(), raw),
            other => panic!("expected Unknown, got {:?}", other),
        }
        assert_eq!(p.to_bytes(), raw);
    }

    #[test]
    fn to_bytes_matches_known_constants() {
        assert_eq!(AlpnProtocol::Http1.to_bytes(), b"http/1.1");
        assert_eq!(AlpnProtocol::Http2.to_bytes(), b"h2");
        assert_eq!(AlpnProtocol::Http3.to_bytes(), b"h3");
        assert_eq!(AlpnProtocol::None.to_bytes(), b"");
    }

    #[test]
    fn is_tls_only_for_known_tls_variants() {
        assert!(AlpnProtocol::Http1.is_tls());
        assert!(AlpnProtocol::Http2.is_tls());
        assert!(AlpnProtocol::Http3.is_tls());
        assert!(!AlpnProtocol::None.is_tls());
        assert!(!AlpnProtocol::Unknown(Bytes::from_static(b"h2c")).is_tls());
    }

    #[test]
    fn helpers_contents_and_order() {
        assert_eq!(alp_h2_h1(), vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
        assert_eq!(alp_h1_h2(), vec![b"http/1.1".to_vec(), b"h2".to_vec()]);
        assert_eq!(alp_h1(), vec![b"http/1.1".to_vec()]);
        assert_eq!(alp_h2(), vec![b"h2".to_vec()]);
        assert_eq!(alp_h3(), vec![b"h3".to_vec()]);
        assert_eq!(
            alp_h3_all(),
            vec![
                b"h3".to_vec(),
                b"h3-32".to_vec(),
                b"h3-31".to_vec(),
                b"h3-30".to_vec(),
                b"h3-29".to_vec(),
            ]
        );
    }

    #[test]
    fn default_clone_eq_behavior() {
        let a = AlpnProtocol::default();
        let b = a.clone();
        assert_eq!(a, AlpnProtocol::None);
        assert_eq!(a, b);

        let u1 = AlpnProtocol::Unknown(Bytes::from_static(b"foo"));
        let u2 = AlpnProtocol::Unknown(Bytes::from_static(b"foo"));
        let u3 = AlpnProtocol::Unknown(Bytes::from_static(b"bar"));
        assert_eq!(u1, u2);
        assert_ne!(u1, u3);
    }

    #[test]
    fn round_trip_knowns() {
        let known = [
            AlpnProtocol::Http1,
            AlpnProtocol::Http2,
            AlpnProtocol::Http3,
            AlpnProtocol::None,
        ];
        for p in known {
            let round = if p == AlpnProtocol::None {
                AlpnProtocol::from_bytes_opt(None)
            } else {
                AlpnProtocol::from_bytes(p.to_bytes())
            };
            assert_eq!(round, p, "round-trip failed for {:?}", p);
        }
    }
}
