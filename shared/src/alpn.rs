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
    vec![ALPN_H2.to_vec(), ALPN_H11.to_vec()] // TODO: make this configurable
}
pub fn alp_h1_h2() -> Vec<Vec<u8>> {
    vec![ALPN_H11.to_vec(), ALPN_H2.to_vec()] // TODO: make this configurable
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
