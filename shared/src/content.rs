use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    ops::Deref,
};

use brotli::enc::BrotliEncoderParams;
use bytes::Bytes;
use cow_utils::CowUtils;
use flate2::{
    Compression, GzBuilder,
    bufread::{DeflateDecoder, DeflateEncoder},
    read::GzDecoder,
};
use http::{
    HeaderMap, HeaderName,
    header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE},
};
use strum::VariantArray;
use tracing::trace;

#[derive(Debug, Clone, PartialEq, Eq, VariantArray)]
pub enum ContentType {
    Bmp,
    Csv,
    Gif,
    Html,
    Jpeg,
    Json,
    Md,
    Png,
    Svg,
    Text,
    Toml,
    Tsv,
    Webp,
    XIcon,
    Xml,
    Yaml,
    OctetStream,
}

const MIME_APPLICATION_CSV: &str = "application/csv";
const MIME_APPLICATION_JSON: &str = "application/json";
const MIME_APPLICATION_OCTECT_STREAM: &str = "application/octet-stream";
const MIME_APPLICATION_TOML: &str = "application/toml";
const MIME_APPLICATION_TSV: &str = "application/tsv";
const MIME_APPLICATION_XML: &str = "application/xml";
const MIME_APPLICATION_YAML: &str = "application/yaml";
const MIME_IMAGE_BMP: &str = "image/bmp";
const MIME_IMAGE_GIF: &str = "image/gif";
const MIME_IMAGE_IICON: &str = "image/i-icon";
const MIME_IMAGE_XICON: &str = "image/x-icon";
const MIME_IMAGE_JPEG: &str = "image/jpeg";
const MIME_IMAGE_PNG: &str = "image/png";
const MIME_IMAGE_SVG_XML: &str = "image/svg+xml";
const MIME_IMAGE_WEBP: &str = "image/webp";
const MIME_TEXT_HTML: &str = "text/html";
const MIME_TEXT_MARKDOWN: &str = "text/markdown";
const MIME_TEXT_PLAIN: &str = "text/plain";

impl ContentType {
    pub fn to_default_str(&self) -> &str {
        match self {
            ContentType::Bmp => MIME_IMAGE_BMP,
            ContentType::Csv => MIME_APPLICATION_CSV,
            ContentType::Gif => MIME_IMAGE_GIF,
            ContentType::Html => MIME_TEXT_HTML,
            ContentType::Jpeg => MIME_IMAGE_JPEG,
            ContentType::Json => MIME_APPLICATION_JSON,
            ContentType::Md => MIME_TEXT_MARKDOWN,
            ContentType::OctetStream => MIME_APPLICATION_OCTECT_STREAM,
            ContentType::Png => MIME_IMAGE_PNG,
            ContentType::Svg => MIME_IMAGE_SVG_XML,
            ContentType::Text => MIME_TEXT_PLAIN,
            ContentType::Toml => MIME_APPLICATION_TOML,
            ContentType::Tsv => MIME_APPLICATION_TSV,
            ContentType::Webp => MIME_IMAGE_WEBP,
            ContentType::XIcon => MIME_IMAGE_IICON,
            ContentType::Xml => MIME_APPLICATION_XML,
            ContentType::Yaml => MIME_APPLICATION_YAML,
        }
    }
}

const EXT_BMP: &str = "bmp";
const EXT_CSV: &str = "csv";
const EXT_GIF: &str = "gif";
const EXT_HTML: &str = "html";
const EXT_ICNS: &str = "icns";
const EXT_ICO: &str = "ico";
const EXT_JPG: &str = "jpg";
const EXT_JPEG: &str = "jpeg";
const EXT_JSON: &str = "json";
const EXT_MD: &str = "md";
const EXT_OCTET_STREAM: &str = "oct";
const EXT_PNG: &str = "png";
const EXT_SVG: &str = "svg";
const EXT_TOML: &str = "toml";
const EXT_TSV: &str = "tsv";
const EXT_TXT: &str = "txt";
const EXT_WEBP: &str = "webp";
const EXT_XML: &str = "xml";
const EXT_YAML: &str = "yaml";

pub fn ext_to_content_type(ext: &str) -> Option<ContentType> {
    match ext {
        EXT_BMP => Some(ContentType::Bmp),
        EXT_CSV => Some(ContentType::Csv),
        EXT_GIF => Some(ContentType::Gif),
        EXT_HTML => Some(ContentType::Html),
        EXT_ICNS => Some(ContentType::XIcon),
        EXT_ICO => Some(ContentType::XIcon),
        EXT_JPG => Some(ContentType::Jpeg),
        EXT_JPEG => Some(ContentType::Jpeg),
        EXT_JSON => Some(ContentType::Json),
        EXT_MD => Some(ContentType::Md),
        EXT_OCTET_STREAM => Some(ContentType::OctetStream),
        EXT_PNG => Some(ContentType::Png),
        EXT_SVG => Some(ContentType::Svg),
        EXT_TOML => Some(ContentType::Toml),
        EXT_TSV => Some(ContentType::Tsv),
        EXT_TXT => Some(ContentType::Text),
        EXT_WEBP => Some(ContentType::Webp),
        EXT_XML => Some(ContentType::Xml),
        EXT_YAML => Some(ContentType::Yaml),
        _ => None,
    }
}
pub fn content_type_ext(content_type: &ContentType) -> &'static str {
    match content_type {
        ContentType::Bmp => EXT_BMP,
        ContentType::Csv => EXT_CSV,
        ContentType::Gif => EXT_GIF,
        ContentType::Html => EXT_HTML,
        ContentType::Jpeg => EXT_JPEG,
        ContentType::Json => EXT_JSON,
        ContentType::Md => EXT_MD,
        ContentType::Png => EXT_PNG,
        ContentType::Svg => EXT_SVG,
        ContentType::Text => EXT_TXT,
        ContentType::Toml => EXT_TOML,
        ContentType::Tsv => EXT_TSV,
        ContentType::Webp => EXT_WEBP,
        ContentType::XIcon => EXT_ICO,
        ContentType::Xml => EXT_XML,
        ContentType::Yaml => EXT_YAML,
        ContentType::OctetStream => EXT_OCTET_STREAM,
    }
}

pub fn parse_content_type(content_type: &str) -> Option<ContentType> {
    let ct = content_type.cow_to_ascii_lowercase();
    match ct.as_ref() {
        MIME_APPLICATION_JSON => Some(ContentType::Json),
        MIME_IMAGE_BMP => Some(ContentType::Bmp),
        MIME_APPLICATION_XML => Some(ContentType::Xml),
        MIME_APPLICATION_CSV => Some(ContentType::Csv),
        MIME_APPLICATION_TSV => Some(ContentType::Tsv),
        MIME_TEXT_MARKDOWN => Some(ContentType::Md),
        MIME_TEXT_HTML => Some(ContentType::Html),
        MIME_APPLICATION_TOML => Some(ContentType::Toml),
        MIME_APPLICATION_YAML => Some(ContentType::Yaml),
        MIME_IMAGE_PNG => Some(ContentType::Png),
        MIME_IMAGE_JPEG => Some(ContentType::Jpeg),
        MIME_APPLICATION_OCTECT_STREAM => Some(ContentType::OctetStream),
        MIME_IMAGE_WEBP => Some(ContentType::Webp),
        MIME_IMAGE_GIF => Some(ContentType::Gif),
        MIME_IMAGE_XICON => Some(ContentType::XIcon),
        MIME_IMAGE_SVG_XML => Some(ContentType::Svg),
        MIME_TEXT_PLAIN => Some(ContentType::Text),
        _ => None,
    }
}

pub fn content_type(headers: &HeaderMap) -> Option<ContentType> {
    let content_type = headers
        .get(CONTENT_TYPE)
        .map(|s| s.to_str().unwrap_or(""))
        .unwrap_or("");
    parse_content_type(content_type)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encodings {
    Gzip,
    Deflate,
    Brotli,
    Zstd,
    // TODO:
    // Content-Encoding: compress
    // Content-Encoding: dcb
    // Content-Encoding: dcz
    // har
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
    trace!(
        "decode body {:?}",
        encoding.iter().rev().collect::<Vec<_>>()
    );

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
    trace!("Encoding body {encoding:?}");

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
