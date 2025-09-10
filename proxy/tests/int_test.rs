use bytes::Bytes;
use chrono::Utc;
use futures::stream::FuturesUnordered;
use futures::{SinkExt, StreamExt};
use http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, DATE, HOST, SET_COOKIE, TE,
    TRANSFER_ENCODING,
};
use http::{HeaderName, Method, Uri, Version};
use http_body_util::Empty;
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use itertools::Itertools;
use roxy_proxy::flow::FlowStore;
use roxy_proxy::interceptor::{ScriptEngine, ScriptType};
use roxy_proxy::proxy::ProxyManager;
use roxy_servers::web_transport::h3_wt;
use roxy_servers::ws::{start_ws_server, start_wss_server};
use roxy_servers::{HttpServers, load_asset};
use roxy_shared::cert::LoggingServerVerifier;
use roxy_shared::client::ClientContext;
use roxy_shared::content::{
    ContentType, Encodings, content_type_ext, decode_body, encode_body, ext_to_content_type,
};
use roxy_shared::h3_client::client_h3_wt;
use roxy_shared::http::HttpResponse;
use roxy_shared::io::local_tcp_listener;
use roxy_shared::tls::TlsConfig;
use roxy_shared::uri::RUri;
use roxy_shared::{RoxyCA, generate_roxy_root_ca_with_path};
use rustls::ClientConfig;
use std::collections::HashSet;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use strum::VariantArray;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{Connector, client_async, connect_async_tls_with_config};
use tracing::error;
use url::form_urlencoded;

static TIMEOUT: u64 = 15_000;
struct TestContext {
    proxy_socket_addr: SocketAddr,
    proxy_addr: RUri,
    roxy_ca: RoxyCA,
    _proxy_manager: ProxyManager,
    script_engine: ScriptEngine,
    tls_config: TlsConfig,
    flow_store: FlowStore,
}

impl TestContext {
    pub async fn new_with_tls(tls_config: Option<TlsConfig>) -> Self {
        roxy_proxy::init_test_logging();
        println!("Starting test context");

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let script_engine = ScriptEngine::new();

        let flow_store = FlowStore::new();
        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_socket_addr = listener.local_addr().unwrap();
        let proxy_port = proxy_socket_addr.port();
        let addr = format!("127.0.0.1:{proxy_port}");
        let proxy_uri: RUri = addr.parse().unwrap();
        let udp_socket = UdpSocket::bind(format!("127.0.0.1:{proxy_port}")).unwrap();

        let tls_config = tls_config.unwrap_or_default();
        let mut proxy_manager = ProxyManager::new(
            0,
            roxy_ca.clone(),
            script_engine.clone(),
            tls_config,
            flow_store.clone(),
        );
        proxy_manager.start_tcp(listener).await.unwrap();
        proxy_manager.start_udp(udp_socket).await.unwrap();

        TestContext {
            proxy_socket_addr,
            proxy_addr: proxy_uri,
            _proxy_manager: proxy_manager,
            roxy_ca,
            script_engine,
            tls_config: TlsConfig::default(),
            flow_store,
        }
    }

    pub async fn new() -> Self {
        TestContext::new_with_tls(None).await
    }

    pub async fn set_script(&mut self, script: &str) -> Result<(), Box<dyn Error>> {
        self.script_engine
            .set_script(script, ScriptType::Lua)
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_connect_rejected() {
    let cxt = TestContext::new().await;
    let client = ClientContext::builder().build();

    let assert_resp = |resp: &HttpResponse, status: u16, version: Version| {
        assert_eq!(resp.parts.status, status);
        assert_eq!(resp.parts.version, version);
        assert!(resp.parts.headers.get(DATE).is_some());
        if status == 400 {
            assert_eq!(resp.parts.extensions.len(), 0);
            assert!(resp.parts.headers.get(CONTENT_LENGTH).is_some());
            assert_eq!(resp.parts.headers.len(), 2);
        } else {
            assert_eq!(resp.parts.extensions.len(), 1);
            assert_eq!(resp.parts.headers.len(), 1);
        }
        assert_eq!(resp.body.len(), 0);
        assert!(resp.trailers.is_none());
    };

    let request = async |version: Version, uri: &str, host: Option<&str>| {
        let mut builder = http::Request::builder()
            .method(Method::CONNECT)
            .version(version)
            .uri(uri);
        if let Some(host) = host {
            builder = builder.header(HOST, host);
        }
        let request = builder.body(BoxBody::new(Empty::new())).unwrap();
        client.request(request).await
    };

    // Valid
    let resp = request(
        Version::HTTP_11,
        &cxt.proxy_addr.host_port(),
        Some(cxt.proxy_addr.host()),
    )
    .await
    .unwrap();
    assert_resp(&resp, 200, Version::HTTP_11);

    // missing host
    let resp = request(Version::HTTP_11, &cxt.proxy_addr.host_port(), None)
        .await
        .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // different host from uri
    let resp = request(
        Version::HTTP_11,
        &cxt.proxy_addr.host_port(),
        Some("localhost"),
    )
    .await
    .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // different port from uri
    let resp = request(
        Version::HTTP_11,
        &cxt.proxy_addr.host_port(),
        Some(&format!("{}:800", cxt.proxy_addr.host())),
    )
    .await
    .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // TODO: not working on windows
    // missing port
    // let resp = request(
    //     Version::HTTP_11,
    //     cxt.proxy_addr.host(),
    //     Some(cxt.proxy_addr.host()),
    // );
    // assert!(resp.await.is_err());
    // assert_eq!(client.request(req).await.unwrap().parts.status, 400);

    // with scheme
    let resp = request(
        Version::HTTP_11,
        &format!("http://{}", cxt.proxy_addr.host_port()),
        Some("localhost"),
    )
    .await
    .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // with path - uri is invalid without http here, not sure how to test
    let resp = request(
        Version::HTTP_11,
        &format!("http://{}/path", cxt.proxy_addr.host_port()),
        Some("localhost"),
    )
    .await
    .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // with query - uri is invalid without http, not sure how to test
    let resp = request(
        Version::HTTP_11,
        &format!("http://{}?foo=bar", cxt.proxy_addr.host_port()),
        Some("localhost"),
    )
    .await
    .unwrap();
    assert_resp(&resp, 400, Version::HTTP_11);

    // H3 error
    let resp = request(
        Version::HTTP_3,
        &cxt.proxy_addr.host_port(),
        Some(cxt.proxy_addr.host()),
    )
    .await;
    assert!(resp.is_err());

    // Valid - at bottom because server responds with http_10 after this
    let resp = request(
        Version::HTTP_10,
        &cxt.proxy_addr.host_port(),
        Some(cxt.proxy_addr.host()),
    )
    .await
    .unwrap();
    assert_resp(&resp, 200, Version::HTTP_10);

    // Not valid in H09
    assert!(
        request(
            Version::HTTP_09,
            &cxt.proxy_addr.host_port(),
            Some(cxt.proxy_addr.host())
        )
        .await
        .is_err()
    );

    assert_eq!(cxt.flow_store.flows.len(), 0)
}

#[tokio::test]
async fn test_http_proxy_request() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(s.target.clone())
            .header(HOST, s.target.host())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.version, s.server.version());
        assert_eq!(parts.status, 200);
        assert_eq!(parts.extensions.len(), 0);

        if s.server.version() != Version::HTTP_3 {
            assert_eq!(parts.headers.len(), 2);
            assert!(parts.headers.get(DATE).is_some());
            assert!(parts.headers.get(CONTENT_LENGTH).is_some());
        } else {
            assert_eq!(parts.headers.len(), 0);
        }

        assert_eq!(body, format!("Hello, {}", s.server.marker()));
        assert!(trailers.is_none());
    }

    assert_eq!(cxt.flow_store.flows.len(), 6)
}

#[tokio::test]
async fn test_http_get_asset() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for (server_index, s) in servers.iter().enumerate() {
        for (content_index, content_type) in ContentType::VARIANTS.iter().enumerate() {
            let uri = format!("{}assets/test.{}", s.target, content_type_ext(content_type));
            let content_type_header = ext_to_content_type(uri.split(".").last().unwrap());

            let req = http::Request::builder()
                .method(Method::GET)
                .version(s.server.version())
                .uri(uri.clone())
                .header(HOST, s.target.host())
                .body(BoxBody::new(Empty::new()))
                .unwrap();

            let client = ClientContext::builder()
                .with_proxy(cxt.proxy_addr.clone())
                .with_roxy_ca(cxt.roxy_ca.clone())
                .with_alpns(vec![s.server.alpn()])
                .build();

            let HttpResponse {
                parts,
                body,
                trailers,
            } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
                .await
                .unwrap()
                .unwrap();

            assert_eq!(parts.version, s.server.version());
            assert_eq!(parts.status, 200);
            assert_eq!(parts.extensions.len(), 0);

            if s.server.version() != Version::HTTP_3 {
                assert_eq!(parts.headers.len(), 3);
                assert!(parts.headers.get(DATE).is_some());
                assert!(parts.headers.get(CONTENT_LENGTH).is_some());
            } else {
                assert_eq!(parts.headers.len(), 1);
            }
            assert_eq!(
                parts.headers.get(CONTENT_TYPE).unwrap().to_str().unwrap(),
                content_type_header.clone().unwrap().to_default_str(),
            );

            let data = Bytes::from(load_asset(content_type).await.unwrap());

            assert_eq!(body, data);
            assert!(trailers.is_none());

            let ids = cxt.flow_store.ordered_ids.read().await;
            let flow_index = (server_index * ContentType::VARIANTS.len()) + content_index;
            let id = ids.get(flow_index).unwrap();
            let flow = cxt.flow_store.flows.get(id).unwrap();
            let flow = flow.read().await;

            let intercept_request = flow.request.clone().unwrap();

            assert_eq!(intercept_request.uri, uri.parse::<RUri>().unwrap());
            assert!(intercept_request.encoding.is_none());
            assert_eq!(intercept_request.alpn, s.server.alpn());
            assert_eq!(intercept_request.method, Method::GET);
            assert_eq!(intercept_request.version, s.server.http_version());
            assert!(intercept_request.body.is_empty());

            if s.server.version() == Version::HTTP_3 {
                assert_eq!(intercept_request.headers.len(), 0);
            } else {
                assert_eq!(intercept_request.headers.len(), 1);
            }
            assert!(intercept_request.body.is_empty());
            assert!(intercept_request.trailers.is_none());

            let intercepted_response = flow.response.clone().unwrap();
            assert_eq!(intercepted_response.status, 200);
            assert_eq!(intercepted_response.version, s.server.http_version());
            if s.server.version() == Version::HTTP_3 {
                assert_eq!(intercepted_response.headers.len(), 1);
            } else {
                assert_eq!(intercepted_response.headers.len(), 2);
                assert!(intercepted_response.headers.get(DATE).is_some());
            }
            assert_eq!(
                intercepted_response
                    .headers
                    .get(CONTENT_TYPE)
                    .unwrap()
                    .to_str()
                    .unwrap(),
                content_type_header.unwrap().to_default_str(),
            );
            assert_eq!(intercepted_response.body, data);
            assert!(intercepted_response.trailers.is_none());

            assert!(flow.error.is_none());
            assert!(flow.messages.is_empty());
        }
    }
}

// fn permutate(provider: CryptoProvider) -> Vec<CryptoProvider> {
//     let mut out = vec![];
//     for cs in &provider.cipher_suites {
//         for kx_group in &provider.kx_groups {
//             // for sva in provider.signature_verification_algorithms { // TODO: permutate
//             out.push({
//                 CryptoProvider {
//                     cipher_suites: vec![*cs],
//                     kx_groups: vec![*kx_group],
//                     signature_verification_algorithms: provider.signature_verification_algorithms,
//                     secure_random: provider.secure_random,
//                     key_provider: provider.key_provider,
//                 }
//             });
//             // }
//         }
//     }
//     out
// }

// #[tokio::test]
// async fn test_tls_permutations() {
//     roxy_proxy::init_test_logging();
//     init_crypto();
//     let crypto_provider = permutate(default_provider());
//
//     for cp in crypto_provider {
//         let tls_config = TlsConfig::from_provider(cp.clone());
//         let cxt = TestContext::new_with_tls(Some(tls_config.clone())).await;
//
//         let mut set = HashSet::new();
//         // set.insert(HttpServers::H10S);
//         set.insert(HttpServers::H11S);
//         set.insert(HttpServers::H2);
//
//         let servers = HttpServers::start_set(set, &cxt.roxy_ca, &tls_config)
//             .await
//             .unwrap();
//
//         for s in &servers {
//             let req = http::Request::builder()
//                 .method(Method::GET)
//                 .version(s.server.version())
//                 .uri(s.target.clone())
//                 .header(HOST, s.target.host())
//                 .body(BoxBody::new(Empty::new()))
//                 .unwrap();
//
//             let client = ClientContext::builder()
//                 .with_proxy(cxt.proxy_addr.clone())
//                 .with_roxy_ca(cxt.roxy_ca.clone())
//                 .with_tls_config(tls_config.clone())
//                 .with_alpns(vec![s.server.alpn()])
//                 .build();
//
//             let HttpResponse {
//                 parts,
//                 body,
//                 trailers,
//             } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
//                 .await
//                 .unwrap()
//                 .unwrap();
//
//             debug!("{parts:?}");
//             assert_eq!(parts.status, 200);
//             assert_eq!(parts.version, s.server.version());
//             assert_eq!(body, format!("Hello, {}", s.server.marker()));
//             assert!(trailers.is_none());
//         }
//
//     }
// }

#[tokio::test]
async fn test_http_proxy_request_async() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    let handles = FuturesUnordered::new();
    for _i in 0..10 {
        for s in &servers {
            let proxy_addr = cxt.proxy_addr.clone();
            let server = s.server;
            let target = s.target.clone();
            let ca = cxt.roxy_ca.clone();

            let h = tokio::spawn(async move {
                let req = http::Request::builder()
                    .method(Method::GET)
                    .version(server.version())
                    .uri(target)
                    .body(BoxBody::new(Empty::new()))
                    .unwrap();

                let client = ClientContext::builder()
                    .with_proxy(proxy_addr)
                    .with_roxy_ca(ca)
                    .build();

                let HttpResponse {
                    parts,
                    body,
                    trailers,
                } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
                    .await
                    .unwrap()
                    .unwrap();

                assert_eq!(parts.version, server.version());
                assert_eq!(parts.status, 200);
                assert_eq!(parts.extensions.len(), 0);

                if server.version() != Version::HTTP_3 {
                    assert_eq!(parts.headers.len(), 2);
                    assert!(parts.headers.get(DATE).is_some());
                    assert!(parts.headers.get(CONTENT_LENGTH).is_some());
                } else {
                    assert_eq!(parts.headers.len(), 0);
                }

                assert_eq!(body, format!("Hello, {}", server.marker()));
                assert!(trailers.is_none());
            });
            handles.push(h);
        }
    }
    handles.for_each(|_| async {}).await;
}

#[tokio::test]
async fn test_http_proxy_request_multiple_cookies() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();
    for s in &servers {
        let mut parts = s.target.inner.clone().into_parts();
        let pq = http::uri::PathAndQuery::from_static("/cookies");
        parts.path_and_query = Some(pq);

        let cookie_a = rnd_string();
        let cookie_b = rnd_string();

        let target = Uri::from_parts(parts).unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(target)
            .header(HOST, s.target.host())
            .header(SET_COOKIE, cookie_a.clone())
            .header(SET_COOKIE, cookie_b.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(parts.version, s.server.version());
        let cookies = parts
            .headers
            .get_all(SET_COOKIE)
            .iter()
            .map(|c| c.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(cookies.len(), 3);

        assert!(cookies.contains(&cookie_a.as_str()));
        assert!(cookies.contains(&cookie_b.as_str()));
        assert!(cookies.contains(&s.server.marker()));

        assert!(body.is_empty());
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_http_proxy_request_query() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let mut parts = s.target.inner.clone().into_parts();

        let encoded: String = form_urlencoded::Serializer::new(String::new())
            .append_pair("foo", "bar & baz")
            .append_pair("saison", "Été+hiver")
            .append_pair("server", s.server.marker())
            .finish();

        let pq = format!("/query?{encoded}");
        let pq: http::uri::PathAndQuery = pq.parse().unwrap();
        parts.path_and_query = Some(pq);

        let target = Uri::from_parts(parts).unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(target)
            .header(HOST, s.target.host())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(parts.version, s.server.version());
        assert!(body.is_empty());
        assert!(trailers.is_none());
    }
}

static INTERCEPT_QUERY_SCRIPT: &str = r#"
Extensions = {
  {
  function (flow) 
      flow.request.url.searchParams["foo"] = "bar & baz"
      flow.request.url.searchParams["saison"] = "Été+hiver"
  end,
  function (flow) 
  end,
  }
}
"#;

#[tokio::test]
async fn test_intercept_http_proxy_request_query() {
    let mut cxt = TestContext::new().await;
    cxt.set_script(INTERCEPT_QUERY_SCRIPT).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let mut parts = s.target.inner.clone().into_parts();

        let encoded: String = form_urlencoded::Serializer::new(String::new())
            .append_pair("server", s.server.marker())
            .finish();

        let pq = format!("/query?{encoded}");
        let pq: http::uri::PathAndQuery = pq.parse().unwrap();
        parts.path_and_query = Some(pq);

        let target = Uri::from_parts(parts).unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(target)
            .header(HOST, s.target.host())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_alpns(vec![s.server.alpn()])
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(parts.version, s.server.version());
        assert!(body.is_empty());
        assert!(trailers.is_none());
    }
}

fn rnd_string() -> String {
    Utc::now().to_string()
}

#[tokio::test]
async fn test_http_proxy_request_compress() {
    let cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let encs = vec![
            Encodings::Gzip,
            Encodings::Deflate,
            Encodings::Brotli,
            Encodings::Zstd,
        ];
        let len = encs.len();
        for enc in encs.into_iter().permutations(len) {
            let mut parts = s.target.inner.clone().into_parts();
            let pq = http::uri::PathAndQuery::from_static("/compress");
            parts.path_and_query = Some(pq);

            let body_str = Utc::now().to_string();
            let rnd = Bytes::from(body_str.clone());
            let body = encode_body(&rnd, &enc).unwrap();
            let body = BoxBody::new(Full::new(body));

            let enc_header = enc.iter().map(|f| f.key()).join(", ");
            let target = Uri::from_parts(parts).unwrap();
            let req = http::Request::builder()
                .method(Method::POST)
                .version(s.server.version())
                .uri(target)
                .header(HOST, s.target.host())
                .header(CONTENT_ENCODING, enc_header.clone())
                .header(ACCEPT_ENCODING, enc_header)
                .body(body)
                .unwrap();

            let client = ClientContext::builder()
                .with_proxy(cxt.proxy_addr.clone())
                .with_roxy_ca(cxt.roxy_ca.clone())
                .with_alpns(vec![s.server.alpn()])
                .build();

            let HttpResponse {
                parts,
                body,
                trailers,
            } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
                .await
                .unwrap()
                .unwrap();

            assert_eq!(parts.status, 200);
            assert_eq!(parts.version, s.server.version());
            let body = decode_body(&body, &enc).unwrap();
            assert_eq!(
                body,
                format!("Hello, {}, pong {}", s.server.marker(), body_str)
            );
            assert!(trailers.is_none());
        }
    }
}

#[tokio::test]
async fn test_http_proxy_chunked() {
    let cxt = TestContext::new().await;
    let mut set = HashSet::new();
    set.insert(HttpServers::H11);
    set.insert(HttpServers::H11S);
    let servers = HttpServers::start_set(set, &cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let mut parts = s.target.inner.clone().into_parts();
        let pq = http::uri::PathAndQuery::from_static("/chunked");
        parts.path_and_query = Some(pq);

        let body_str = Utc::now().to_string();
        let rnd = Bytes::from(body_str.clone());
        let body = BoxBody::new(Full::new(rnd));

        let target = Uri::from_parts(parts).unwrap();
        let req = http::Request::builder()
            .method(Method::POST)
            .version(s.server.version())
            .uri(target)
            .header(HOST, s.target.host())
            .header(TRANSFER_ENCODING, "chunked")
            .body(body)
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(parts.version, s.server.version());
        assert_eq!(
            body,
            format!("Hello, {}, pong {}", s.server.marker(), body_str)
        );
        assert!(trailers.is_none());
    }
}

static REWRITE_BODY: &str =
    "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>";

static REWRITE_SCRIPT: &str = r#"
Extensions = {
  {
  function (flow) 
  end,
  function (flow) 
    flow.response.body.text = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
  end,
  }
}
"#;

#[tokio::test]
async fn test_rewrite_http_response_body() {
    let mut cxt = TestContext::new().await;
    cxt.set_script(REWRITE_SCRIPT).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(s.target.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(body, REWRITE_BODY);
        assert!(trailers.is_none());
    }
}

static RETURN_BODY_EARLY: &str = r#"
Extensions = {
  {
  function (flow) 
    flow.response.body.text = "Early return"
  end,
  function (flow) 
  end,
  }
}
"#;

#[tokio::test]
async fn test_early_return_with_body() {
    let mut cxt = TestContext::new().await;
    cxt.set_script(RETURN_BODY_EARLY).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .header("server_id", s.server.marker())
            .uri(s.target.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .with_alpns(vec![s.server.alpn()])
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(body, "Early return");
        // assert_eq!(
        //     parts.headers.get("server_id").unwrap().to_str().unwrap(),
        //     s.server.marker()
        // );
        assert!(trailers.is_none());
    }
}

static GSUB_BODY_SCRIPT: &str = r#"
function req(flow) 
    flow.request.body.text = string.gsub(flow.request.body.text, "replaceme", "gone")
end

function resp(flow) 
    flow.response.body.text = string.gsub(flow.response.body.text, "to_go", "it_went")
end
Extensions = {
  {
    request = req,
    response = resp
  },
}
"#;

#[tokio::test]
async fn test_gsub_body() {
    let mut cxt = TestContext::new().await;
    cxt.set_script(GSUB_BODY_SCRIPT).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let mut parts = s.target.inner.clone().into_parts();
        let pq = http::uri::PathAndQuery::from_static("/gsub");
        parts.path_and_query = Some(pq);
        let target = Uri::from_parts(parts).unwrap();
        let req_body = Bytes::from_static(b"this replaceme needs to go");
        let req = http::Request::builder()
            .method(Method::POST)
            .version(s.server.version())
            .uri(target)
            .body(BoxBody::new(Full::new(req_body)))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        let expect = format!("this it_went needs to go {}", s.server.marker());

        assert_eq!(parts.status, 200);
        assert_eq!(body, Bytes::from(expect));
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_http_proxy_request_trailers() {
    let cxt = TestContext::new().await;

    let mut set = HashSet::new();
    set.insert(HttpServers::H11);
    set.insert(HttpServers::H11S);
    set.insert(HttpServers::H2);
    set.insert(HttpServers::H3);

    let servers = HttpServers::start_set(set, &cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();
    for s in &servers {
        let target_uri: RUri = format!("{}://{}/trailers", s.target.scheme(), s.target.host_port())
            .parse()
            .unwrap();

        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .header(TE, "trailers")
            .uri(target_uri.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        assert_eq!(body, "trailers");

        let trailers = trailers.unwrap();
        assert_eq!(trailers.len(), 1);

        let h = HeaderName::from_bytes(b"hello").unwrap();
        assert_eq!(trailers[h], "world");
    }
}

#[tokio::test]
async fn test_redirect_scheme() {
    let mut cxt = TestContext::new().await;
    let mut servers = HttpServers::set_all();
    servers.remove(&HttpServers::H3);
    // TODO: add test that h3 fails

    let servers = HttpServers::start_set(servers, &cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let script = format!(
            r#"
Extensions = {{
  {{
  function (flow) 
    flow.request.url.scheme = "{}"
  end,
  function (flow) 
  end,
  }}
}}
    "#,
            s.target.scheme()
        );

        cxt.set_script(&script).await.unwrap();

        let (scheme, version) = if s.server.is_tls() {
            ("http", Version::HTTP_11)
        } else {
            ("https", s.server.version())
        };

        let target_uri: RUri = format!("{}://{}", scheme, s.target.host_port())
            .parse()
            .unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(version)
            .header(HOST, s.target.host())
            .uri(target_uri.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        let server_id = s.server.marker();
        let expected = format!("Hello, {server_id}");
        assert_eq!(body, expected);
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_redirect_http_request() {
    let mut cxt = TestContext::new().await;
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let server_port = s.target.port();
        let script = format!(
            r#"
Extensions = {{
  {{
  function (flow) 
    flow.request.url.host = "127.0.0.1"
    flow.request.url.port = {server_port}
  end,
  function (flow) 
  end,
  }}
}}
        "#,
        );
        cxt.set_script(&script).await.unwrap();
        let target_uri: RUri = format!("{}://idk:8032", s.target.scheme()).parse().unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(target_uri.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        let server_id = s.server.marker();
        let expected = format!("Hello, {server_id}");
        assert_eq!(body, expected);
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_change_path_http_request() {
    let script = r#"
Extensions = {
  {
  function (flow) 
    flow.request.url.path = ""
  end,
  function (flow) 
  end,
  }
}
"#;
    let mut cxt = TestContext::new().await;
    cxt.set_script(script).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let target_uri: RUri = format!("{}/missing", s.target.host_port_scheme())
            .parse()
            .unwrap();
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(target_uri.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        let server_id = s.server.marker();
        let expected = format!("Hello, {server_id}");
        assert_eq!(body, expected);
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_add_header_http_request() {
    let script = r#"
Extensions = {
  {
  function (flow) 
  end,
  function (flow) 
    flow.response.headers['hi'] = 'there'
  end,
  }
}
"#;

    let mut cxt = TestContext::new().await;
    cxt.set_script(script).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(s.target.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        let server_id = s.server.marker();
        let expected = format!("Hello, {server_id}");
        assert_eq!(body, expected);

        let header_v = parts.headers.get("hi").unwrap();
        assert_eq!(header_v, "there");
        assert!(trailers.is_none());
    }
}

#[tokio::test]
async fn test_remove_header_http_request() {
    let script = r#"
Extensions = {
  {
  function (flow) 
  end,
  function (flow) 
    flow.response.headers['test'] = None
  end,
  }
}
"#;
    let mut cxt = TestContext::new().await;
    cxt.set_script(script).await.unwrap();
    let servers = HttpServers::start_all(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    for s in &servers {
        let req = http::Request::builder()
            .method(Method::GET)
            .version(s.server.version())
            .uri(s.target.clone())
            .body(BoxBody::new(Empty::new()))
            .unwrap();

        let client = ClientContext::builder()
            .with_proxy(cxt.proxy_addr.clone())
            .with_roxy_ca(cxt.roxy_ca.clone())
            .build();

        let HttpResponse {
            parts,
            body,
            trailers,
        } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(parts.status, 200);
        let server_id = s.server.marker();
        let expected = format!("Hello, {server_id}");
        assert_eq!(body, expected);

        let header_v = parts.headers.get("test");
        assert_eq!(header_v, None);
        assert!(trailers.is_none());
        error!("success {}", s.server.marker());
    }
}

#[tokio::test]
async fn down_grade_http2_http1() {
    let cxt = TestContext::new().await;

    let s = HttpServers::H11S
        .start(&cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    let req = http::Request::builder()
        .method(Method::GET)
        .version(Version::HTTP_2)
        .uri(s.target.clone())
        .body(BoxBody::new(Empty::new()))
        .unwrap();

    let client = ClientContext::builder()
        .with_proxy(cxt.proxy_addr.clone())
        .with_roxy_ca(cxt.roxy_ca.clone())
        .build();

    let HttpResponse {
        parts,
        body,
        trailers,
    } = timeout(Duration::from_millis(TIMEOUT), client.request(req))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(parts.status, 200);
    let server_id = s.server.marker();
    assert_eq!(body, format!("Hello, {server_id}"));
    assert!(trailers.is_none());
}

#[tokio::test]
async fn ws_test() {
    let cxt = TestContext::new().await;

    let tcp = local_tcp_listener(None).await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let port = addr.port();
    let handle = start_ws_server(tcp).await.unwrap();
    let ws_addr = format!("ws://127.0.0.1:{port}");
    let target_host = format!("127.0.0.1:{port}");

    let mut proxy_stream = TcpStream::connect(cxt.proxy_socket_addr).await.unwrap();
    let connect_req = format!("CONNECT {target_host} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

    proxy_stream
        .write_all(connect_req.as_bytes())
        .await
        .unwrap();

    let mut buf = [0u8; 4096];
    let n = proxy_stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    if !resp.contains("200 ") {
        panic!("Proxy CONNECT failed: {resp}");
    }

    let (mut ws_stream, _ws_read) = client_async(ws_addr, proxy_stream).await.unwrap();

    ws_stream
        .send(Message::Text("Hello, server!".into()))
        .await
        .unwrap();

    if let Some(msg) = ws_stream.next().await {
        match msg.unwrap() {
            Message::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Bad message"),
        }
    }
    handle.abort();
}

#[tokio::test]
async fn wss_test() {
    let cxt = TestContext::new().await;

    let tcp = local_tcp_listener(None).await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let port = addr.port();
    let server_handle = start_wss_server(tcp, &cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    let ws_addr = format!("wss://127.0.0.1:{port}");
    let target_host = format!("127.0.0.1:{port}");

    let mut proxy_stream = TcpStream::connect(cxt.proxy_socket_addr).await.unwrap();

    // TODO: use upgrades
    let connect_req = format!("CONNECT {target_host} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

    proxy_stream
        .write_all(connect_req.as_bytes())
        .await
        .unwrap();

    let mut buf = [0u8; 4096];
    let n = proxy_stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    if !resp.contains("200 ") {
        panic!("Proxy CONNECT failed: {resp}");
    }

    let req = ws_addr.clone().into_client_request().unwrap();

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LoggingServerVerifier::new()))
        .with_no_client_auth();

    let (mut server_ws_stream, _) =
        connect_async_tls_with_config(req, None, false, Some(Connector::Rustls(Arc::new(config))))
            .await
            .unwrap();

    server_ws_stream
        .send(Message::Text("Hello server!".into()))
        .await
        .unwrap();

    if let Some(msg) = server_ws_stream.next().await {
        match msg.unwrap() {
            Message::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Bad message"),
        }
    }

    server_handle.abort();
}

#[tokio::test]
async fn wss_test_h2() {
    let cxt = TestContext::new().await;

    let tcp = local_tcp_listener(None).await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let port = addr.port();
    let server_handle = start_wss_server(tcp, &cxt.roxy_ca, &cxt.tls_config)
        .await
        .unwrap();

    let ws_addr = format!("wss://127.0.0.1:{port}");
    let target_host = format!("127.0.0.1:{port}");

    let mut proxy_stream = TcpStream::connect(cxt.proxy_socket_addr).await.unwrap();

    let connect_req = format!("CONNECT {target_host} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

    proxy_stream
        .write_all(connect_req.as_bytes())
        .await
        .unwrap();

    let mut buf = [0u8; 4096];
    let n = proxy_stream.read(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    if !resp.contains("200 ") {
        panic!("Proxy CONNECT failed: {resp}");
    }

    let req = ws_addr.clone().into_client_request().unwrap();

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LoggingServerVerifier::new()))
        .with_no_client_auth();

    let (mut server_ws_stream, _) =
        connect_async_tls_with_config(req, None, false, Some(Connector::Rustls(Arc::new(config))))
            .await
            .unwrap();

    server_ws_stream
        .send(Message::Text("Hello server!".into()))
        .await
        .unwrap();

    if let Some(msg) = server_ws_stream.next().await {
        match msg.unwrap() {
            Message::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Bad message"),
        }
    }

    server_handle.abort();
}

// TODO: impl wt
#[tokio::test]
async fn test_wt() {
    let cxt = TestContext::new().await;

    let server = h3_wt(&cxt.roxy_ca).await.unwrap();
    let server_port = server.0.port();

    let wt_addr: RUri = format!("https://127.0.0.1:{server_port}").parse().unwrap();

    client_h3_wt(None, &wt_addr, cxt.roxy_ca.roots())
        .await
        .unwrap();
}
