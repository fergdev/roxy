use cow_utils::CowUtils;
use http::{HeaderMap, header::CONTENT_TYPE};
use strum::VariantArray;

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
            ContentType::XIcon => MIME_IMAGE_XICON,
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
        .map(|header_value| header_value.to_str().unwrap_or(""))
        .unwrap_or("");
    let mime_type = content_type
        .split(";")
        .find(|content_type| {
            !content_type.starts_with("boundary=") || content_type.starts_with("charset=")
        })
        .unwrap_or("");
    parse_content_type(mime_type)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue};

    fn headers(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn all_content_types_parse() {
        let cases = [
            ("application/csv", ContentType::Csv),
            ("application/json", ContentType::Json),
            ("application/octet-stream", ContentType::OctetStream),
            ("application/toml", ContentType::Toml),
            ("application/tsv", ContentType::Tsv),
            ("application/xml", ContentType::Xml),
            ("application/yaml", ContentType::Yaml),
            ("image/bmp", ContentType::Bmp),
            ("image/gif", ContentType::Gif),
            ("image/jpeg", ContentType::Jpeg),
            ("image/png", ContentType::Png),
            ("image/svg+xml", ContentType::Svg),
            ("image/webp", ContentType::Webp),
            ("image/x-icon", ContentType::XIcon),
            ("text/html", ContentType::Html),
            ("text/markdown", ContentType::Md),
            ("text/plain", ContentType::Text),
        ];

        for (mime, expected) in cases {
            assert_eq!(parse_content_type(mime), Some(expected));
        }
    }

    #[test]
    fn all_extensions_parse() {
        let cases = [
            ("bmp", ContentType::Bmp),
            ("csv", ContentType::Csv),
            ("gif", ContentType::Gif),
            ("html", ContentType::Html),
            ("icns", ContentType::XIcon),
            ("ico", ContentType::XIcon),
            ("jpg", ContentType::Jpeg),
            ("jpeg", ContentType::Jpeg),
            ("json", ContentType::Json),
            ("md", ContentType::Md),
            ("oct", ContentType::OctetStream),
            ("png", ContentType::Png),
            ("svg", ContentType::Svg),
            ("toml", ContentType::Toml),
            ("tsv", ContentType::Tsv),
            ("txt", ContentType::Text),
            ("webp", ContentType::Webp),
            ("xml", ContentType::Xml),
            ("yaml", ContentType::Yaml),
        ];

        for (ext, expected) in cases {
            assert_eq!(ext_to_content_type(ext), Some(expected));
        }
    }

    #[test]
    fn all_content_types_have_expected_default_mime_and_extension() {
        let cases = [
            (ContentType::Bmp, "image/bmp", "bmp"),
            (ContentType::Csv, "application/csv", "csv"),
            (ContentType::Gif, "image/gif", "gif"),
            (ContentType::Html, "text/html", "html"),
            (ContentType::Jpeg, "image/jpeg", "jpeg"),
            (ContentType::Json, "application/json", "json"),
            (ContentType::Md, "text/markdown", "md"),
            (ContentType::Png, "image/png", "png"),
            (ContentType::Svg, "image/svg+xml", "svg"),
            (ContentType::Text, "text/plain", "txt"),
            (ContentType::Toml, "application/toml", "toml"),
            (ContentType::Tsv, "application/tsv", "tsv"),
            (ContentType::Webp, "image/webp", "webp"),
            (ContentType::XIcon, "image/x-icon", "ico"),
            (ContentType::Xml, "application/xml", "xml"),
            (ContentType::Yaml, "application/yaml", "yaml"),
            (ContentType::OctetStream, "application/octet-stream", "oct"),
        ];

        for (content_type, expected_mime, expected_ext) in cases {
            assert_eq!(content_type.to_default_str(), expected_mime);
            assert_eq!(content_type_ext(&content_type), expected_ext);
        }
    }

    #[test]
    fn content_type_header_parsing_handles_all_supported_types() {
        let cases = [
            ("application/csv; charset=utf-8", Some(ContentType::Csv)),
            ("application/json; charset=utf-8", Some(ContentType::Json)),
            ("application/octet-stream", Some(ContentType::OctetStream)),
            ("application/toml", Some(ContentType::Toml)),
            ("application/tsv", Some(ContentType::Tsv)),
            ("application/xml", Some(ContentType::Xml)),
            ("application/yaml", Some(ContentType::Yaml)),
            ("image/bmp", Some(ContentType::Bmp)),
            ("image/gif", Some(ContentType::Gif)),
            ("image/jpeg", Some(ContentType::Jpeg)),
            ("image/png", Some(ContentType::Png)),
            ("image/svg+xml", Some(ContentType::Svg)),
            ("image/webp", Some(ContentType::Webp)),
            ("image/x-icon", Some(ContentType::XIcon)),
            ("text/html; charset=utf-8", Some(ContentType::Html)),
            ("text/markdown", Some(ContentType::Md)),
            ("text/plain; charset=utf-8", Some(ContentType::Text)),
            ("application/pdf", None),
            ("", None),
        ];

        for (header_value, expected) in cases {
            assert_eq!(content_type(&headers(header_value)), expected);
        }

        assert_eq!(content_type(&HeaderMap::new()), None);
    }

    #[test]
    fn parse_content_type_is_case_insensitive() {
        let cases = [
            ("APPLICATION/JSON", Some(ContentType::Json)),
            ("Text/Html", Some(ContentType::Html)),
            ("IMAGE/PNG", Some(ContentType::Png)),
            ("Image/Svg+Xml", Some(ContentType::Svg)),
        ];

        for (mime, expected) in cases {
            assert_eq!(parse_content_type(mime), expected);
        }
    }

    #[test]
    fn unsupported_values_return_none() {
        assert_eq!(parse_content_type("application/pdf"), None);
        assert_eq!(parse_content_type("multipart/form-data"), None);
        assert_eq!(parse_content_type(""), None);

        assert_eq!(ext_to_content_type("pdf"), None);
        assert_eq!(ext_to_content_type("exe"), None);
        assert_eq!(ext_to_content_type(""), None);
    }
}
