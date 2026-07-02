use bytes::Bytes;
use http::HeaderMap;
use http::StatusCode;
use http::Uri;
use http::Version;
use http::header::HOST;
use http_body_util::Empty;
use http_body_util::combinators::BoxBody;
use tracing::error;
use tracing::trace;

use hyper::Response;
use std::convert::Infallible;

/// https://httpwg.org/specs/rfc9110.html#CONNECT
/// Validate only host and maybe port is provided, anything else is not valid CONNECT
pub(crate) fn validate_connect_uri(version: Version, uri: &Uri, headers: &HeaderMap) -> bool {
    trace!("Validate connect {version:?}, {uri}, {headers:?}");
    let header_host = match headers
        .get(HOST)
        .and_then(|host_header| host_header.to_str().ok())
        .and_then(|host_uri_str| host_uri_str.parse::<Uri>().ok())
    {
        Some(host) => host,
        None => {
            error!(
                "Unable to find host header in CONNECT request uri='{uri}' version='{version:?}'"
            );
            return false;
        }
    };

    let Some(uri_authority) = uri.authority() else {
        return false;
    };
    let Some(uri_port) = uri_authority.port_u16() else {
        return false;
    };
    let Some(header_authority) = header_host.authority() else {
        return false;
    };

    if uri_authority.host() != header_authority.host() {
        return false;
    }
    if let Some(port) = header_authority.port_u16()
        && uri_port != port
    {
        return false;
    }

    if !uri
        .authority()
        .map(|authority| {
            authority.port_u16().is_some() && Some(authority.host()) == header_host.host()
        })
        .unwrap_or(false)
    {
        error!("host uri: {uri} header: {header_host}");
        return false;
    }
    uri.scheme().is_none()
        && uri.path().is_empty()
        && uri.query().is_none()
        && version != Version::HTTP_3
}

pub(crate) fn bad_connect_response() -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(BoxBody::new(Empty::<Bytes>::new()))
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    fn headers_with_host(host: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static(host));
        headers
    }

    #[test]
    fn valid_connect_uri_with_matching_host_and_port() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = headers_with_host("example.com:443");

        assert!(validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn valid_connect_uri_with_matching_host_and_host_header_without_port() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = headers_with_host("example.com");

        assert!(validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_when_host_header_missing() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = HeaderMap::new();

        assert!(!validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_when_uri_has_no_port() {
        let uri: Uri = "example.com".parse().unwrap();
        let headers = headers_with_host("example.com");

        assert!(!validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_when_uri_host_and_header_host_differ() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = headers_with_host("other.com:443");

        assert!(!validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_when_uri_port_and_header_port_differ() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = headers_with_host("example.com:8443");

        assert!(!validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_when_uri_has_scheme() {
        let uri: Uri = "https://example.com:443".parse().unwrap();
        let headers = headers_with_host("example.com:443");

        assert!(!validate_connect_uri(Version::HTTP_11, &uri, &headers));
    }

    #[test]
    fn invalid_for_http_3() {
        let uri: Uri = "example.com:443".parse().unwrap();
        let headers = headers_with_host("example.com:443");

        assert!(!validate_connect_uri(Version::HTTP_3, &uri, &headers));
    }

    #[test]
    fn bad_connect_response_returns_400() {
        let response = bad_connect_response().unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
