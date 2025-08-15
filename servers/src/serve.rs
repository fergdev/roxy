use std::convert::Infallible;

use bytes::Bytes;
use http::{
    HeaderMap, Method, Request, Response, StatusCode,
    header::{CONTENT_ENCODING, CONTENT_TYPE, SET_COOKIE, TE, TRAILER},
    request::Parts,
};
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use iter_tools::Itertools;
use roxy_shared::{
    body::BufferedBody,
    content::{
        decode_body, encode_body, ext_to_content_type, get_accept_enconding, get_content_encoding,
    },
};
use tracing::{debug, info};
use url::Url;

use crate::{HttpServers, load_asset};

pub async fn serve(
    request: Request<hyper::body::Incoming>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let (parts, body) = request.into_parts();
    let body = match body.collect().await {
        Ok(body) => body,
        Err(e) => {
            return Response::builder()
                .status(500)
                .version(parts.version)
                .body(BoxBody::new(Full::from(format!(
                    "Error receiving body {e}"
                ))));
        }
    };
    let t = body.trailers().cloned();

    info!("Server {server}");
    let resp = serve_internal(parts, body.to_bytes(), t, server).await;
    info!("Resp {server} {resp:?}");
    resp
}

pub async fn serve_internal(
    parts: Parts,
    body: Bytes,
    trailers: Option<HeaderMap>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let path = parts.uri.path();
    info!("Path {}", path);

    if path.starts_with("/assets") {
        return handle_asset(parts, body, trailers).await;
    }

    match path {
        "/chunked" => handle_chunked(body, server),
        "/trailers" => handle_trailers(),
        "/compress" => handle_compress(parts, body, trailers, server),
        "/cookies" => handle_cookie(parts, body, trailers, server),
        "/query" => handle_query(parts, body, trailers, server),
        "/gsub" => handle_gsub(parts, body, trailers, server),
        "/" => handle_root(server),
        _ => handle_not_found(),
    }
}

fn handle_root(server: HttpServers) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let body = BoxBody::new(Full::new(Bytes::from(format!(
        "Hello, {}",
        server.marker()
    ))));
    Response::builder().body(body)
}

fn handle_trailers() -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let mut trailers = HeaderMap::new();
    trailers.append("hello", "world".parse()?);
    let body = BufferedBody::with_trailers(Bytes::from("trailers"), trailers);
    Response::builder()
        .header(TRAILER, "hello")
        .header(TE, "trailers")
        .body(BoxBody::new(body))
}

fn handle_chunked(
    body: Bytes,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let mut trailers = HeaderMap::new();
    trailers.append("hello", "world".parse()?);
    let body = BufferedBody::with_bufs(
        vec![
            Bytes::from("Hello, "),
            Bytes::from(server.marker().to_string()),
            Bytes::from(", pong "),
            body,
        ],
        trailers,
    );
    Response::builder()
        .header(TRAILER, "hello")
        .header(TE, "trailers")
        .body(BoxBody::new(body))
}

fn handle_cookie(
    parts: Parts,
    _body: Bytes,
    _trailers: Option<HeaderMap>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let cookies = parts.headers.get_all(SET_COOKIE).iter().collect::<Vec<_>>();
    if cookies.len() != 2 {
        return handle_bad_request("2 cookies required");
    }
    let mut headers = parts.headers;
    headers.append(SET_COOKIE, server.marker().parse()?);

    let mut resp = Response::builder();
    for v in headers.get_all(SET_COOKIE) {
        resp = resp.header(SET_COOKIE, v);
    }
    resp.body(BoxBody::new(Empty::new()))
}

fn handle_compress(
    parts: Parts,
    body: Bytes,
    _trailers: Option<HeaderMap>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let content_encoding = match get_content_encoding(&parts.headers) {
        Some(enc) => enc,
        None => {
            return handle_bad_request("content encoding required");
        }
    };
    let accept_encoding = match get_accept_enconding(&parts.headers) {
        Some(enc) => enc,
        None => return handle_bad_request("accept  encoding required"),
    };

    if accept_encoding != content_encoding {
        return handle_bad_request("accept  encoding required");
    }
    info!("enc {content_encoding:?}");

    let body = if parts.method == Method::POST {
        match decode_body(&body, &content_encoding) {
            Ok(b) => Bytes::from(format!(
                "Hello, {}, pong {}",
                server.marker(),
                String::from_utf8_lossy(&b)
            )),
            Err(_) => {
                return server_error();
            }
        }
    } else {
        Bytes::from(format!("Hello, {}", server.marker()))
    };

    let body = match encode_body(&body, &content_encoding) {
        Ok(b) => b,
        Err(_) => {
            return server_error();
        }
    };

    let enc_header = &content_encoding.iter().map(|f| f.key()).join(", ");
    let resp = Response::builder()
        .header(CONTENT_ENCODING, enc_header)
        .body(BoxBody::new(Full::new(body)))?;
    Ok(resp)
}

fn handle_query(
    parts: Parts,
    _body: Bytes,
    _trailers: Option<HeaderMap>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    info!("query");
    let url = match parts.uri.to_string().parse() {
        Ok(url) => url,
        Err(e) => return handle_bad_request(&format!("invalid path {e}")),
    };

    match assert_query(&url, server.marker()) {
        Ok(_) => Response::builder().body(BoxBody::new(Empty::new())),
        Err(e) => handle_bad_request(&e),
    }
}

async fn handle_asset(
    parts: Parts,
    _body: Bytes,
    _trailers: Option<HeaderMap>,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let path_parts = parts.uri.path().split("/").collect::<Vec<_>>();
    debug!("Parts {path_parts:?}");

    if path_parts.len() != 3 || path_parts[1] != "assets" {
        return handle_bad_request(&format!("Wrong uri parts {path_parts:?}"));
    }

    let ext = match path_parts[2].split(".").nth(1) {
        Some(ext) => ext,
        None => return handle_bad_request("can't find ext"),
    };

    let content_type = match ext_to_content_type(ext) {
        Some(content_type) => content_type,
        None => return handle_bad_request("Bad content type"),
    };

    debug!("Ext {content_type:?}");

    let data = match load_asset(&content_type).await {
        Ok(data) => data,
        Err(e) => return handle_bad_request(&format!("Error loading asset {e}")),
    };
    Response::builder()
        .header(CONTENT_TYPE, content_type.to_default_str())
        .body(BoxBody::new(Full::from(Bytes::from(data))))
}

fn assert_query(url: &Url, marker: &str) -> Result<(), String> {
    let query = url.query_pairs();
    if query.count() != 3 {
        return Err(format!("wrong query count {}", query.count()));
    }
    for q in query {
        if q.0 == "foo" && q.1 == "bar & baz"
            || q.0 == "saison" && q.1 == "Été+hiver"
            || q.0 == "server" && q.1 == marker
        {
            continue;
        } else {
            return Err(format!("bad pair {q:?}"));
        }
    }
    Ok(())
}

fn handle_gsub(
    parts: Parts,
    body: Bytes,
    _trailers: Option<HeaderMap>,
    server: HttpServers,
) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    if parts.method != Method::POST {
        return handle_method_not_allowed();
    }
    if body != Bytes::from_static(b"this gone needs to go") {
        return handle_bad_request("wrong body");
    }

    let body = format!("this to_go needs to go {}", server.marker());
    let resp_body = Bytes::from(body);

    Response::builder().body(BoxBody::new(Full::new(resp_body)))
}

fn handle_method_not_allowed() -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(BoxBody::new(Empty::new()))
}

fn handle_not_found() -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(BoxBody::new(Empty::new()))
}

fn handle_bad_request(msg: &str) -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    let body = BoxBody::new(Full::from(msg.as_bytes().to_owned()));
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
}

fn server_error() -> http::Result<Response<BoxBody<Bytes, Infallible>>> {
    Response::builder()
        .status(500)
        .body(BoxBody::new(Empty::new()))
}
