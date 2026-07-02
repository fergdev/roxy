use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    ops::Deref,
};

use brotli::enc::BrotliEncoderParams;
use bytes::Bytes;
use flate2::{
    Compression, GzBuilder,
    bufread::{DeflateDecoder, DeflateEncoder},
    read::GzDecoder,
};
use http::{
    HeaderMap, HeaderName,
    header::{ACCEPT_ENCODING, CONTENT_ENCODING},
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encodings {
    Gzip,
    Deflate,
    Brotli,
    Zstd,
}

const GZIP: &str = "gzip";
const DEFLATE: &str = "deflate";
const BROTLI: &str = "br";
const ZSTD: &str = "zstd";

impl Encodings {
    pub fn key(&self) -> &str {
        match self {
            Encodings::Gzip => GZIP,
            Encodings::Deflate => DEFLATE,
            Encodings::Brotli => BROTLI,
            Encodings::Zstd => ZSTD,
        }
    }
}

impl Display for Encodings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}
pub fn get_content_encoding(headers: &HeaderMap) -> Option<Vec<Encodings>> {
    get_enconding(CONTENT_ENCODING, headers)
}

pub fn get_accept_enconding(headers: &HeaderMap) -> Option<Vec<Encodings>> {
    get_enconding(ACCEPT_ENCODING, headers)
}

pub fn get_enconding(header_name: HeaderName, headers: &HeaderMap) -> Option<Vec<Encodings>> {
    headers
        .get(header_name)
        .map(|ce| ce.to_str().unwrap_or(""))
        .map(|f| {
            let v = f
                .split(",")
                .filter_map(|f| match f.trim() {
                    GZIP => Some(Encodings::Gzip),
                    DEFLATE => Some(Encodings::Deflate),
                    BROTLI => Some(Encodings::Brotli),
                    ZSTD => Some(Encodings::Zstd),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if v.is_empty() { None } else { Some(v) }
        })
        .unwrap_or(None)
}

pub fn decode_body(body: &Bytes, encoding: &[Encodings]) -> Result<Bytes, Box<dyn Error>> {
    if encoding.is_empty() {
        return Err(Box::new(std::io::Error::other("Empty encoding")));
    }

    let mut body = body.clone();

    for enc in encoding.iter().rev() {
        match enc {
            Encodings::Gzip => {
                let mut result = Vec::new();
                GzDecoder::new(&body[..]).read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
            Encodings::Deflate => {
                let mut result = Vec::new();
                DeflateDecoder::new(&body[..]).read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
            Encodings::Brotli => {
                let mut result = Vec::new();
                brotli::Decompressor::new(&body[..], 4096).read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
            Encodings::Zstd => {
                let mut result = Vec::new();
                zstd::Decoder::new(&body[..])?.read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
        }
    }
    Ok(body)
}

pub fn decode_body_opt(
    body: Bytes,
    encoding: &Option<Vec<Encodings>>,
) -> Result<Bytes, Box<dyn Error>> {
    match encoding {
        Some(enc) => decode_body(&body, enc),
        None => Ok(body),
    }
}

pub fn encode_body(body: &Bytes, encoding: &[Encodings]) -> Result<Bytes, Box<dyn Error>> {
    if encoding.is_empty() {
        return Err(Box::new(std::io::Error::other("Empty encoding")));
    }

    let mut body = body.clone();

    for enc in encoding {
        match enc {
            Encodings::Gzip => {
                let mut result = Vec::new();
                let mut gz = GzBuilder::new()
                    .operating_system(3)
                    .read(&body[..], Compression::default());

                gz.read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
            Encodings::Deflate => {
                let mut result = Vec::new();
                DeflateEncoder::new(&body[..], Compression::default()).read_to_end(&mut result)?;
                body = Bytes::from(result);
            }
            Encodings::Brotli => {
                let mut result = Vec::new();
                brotli::BrotliCompress(
                    &mut body.deref(),
                    &mut result,
                    &BrotliEncoderParams::default(),
                )?;
                body = Bytes::from(result);
            }
            Encodings::Zstd => {
                let result = Vec::new();
                let mut enc = zstd::Encoder::new(result, 0)?;
                enc.write_all(&body[..])?;
                let result = enc.finish()?;
                body = Bytes::from(result);
            }
        }
    }
    Ok(body)
}

pub fn encode_body_opt(
    body: Bytes,
    encoding: &Option<Vec<Encodings>>,
) -> Result<Bytes, Box<dyn Error>> {
    match encoding {
        Some(enc) => encode_body(&body, enc),
        None => Ok(body),
    }
}
