use futures::{SinkExt, StreamExt};
use http::header::HOST;
use http::{Method, StatusCode, Uri};
use http_body_util::BodyExt;
use http3_cli::h3_with_proxy;
use once_cell::sync::OnceCell;
use roxy_proxy::cert::LoggingCertVerifier;
use roxy_proxy::flow::FlowStore;
use roxy_proxy::interceptor::{self};
use roxy_proxy::proxy::start_proxy;
use roxy_servers::h3::default_h3;
use roxy_servers::ws::{start_ws_server, start_wss_server};
use roxy_servers::{start_https_warp_server, start_warp_test_server};
use roxy_shared::{RoxyCA, generate_roxy_root_ca_with_path, init_crypto};
use rustls::ClientConfig;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinHandle as TokioJoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{Connector, client_async, connect_async_tls_with_config};
use warp::Filter;

pub static INIT_LOGGER: OnceCell<()> = OnceCell::new();

pub fn init_logging() {
    INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .init();
    });
}

struct TestContext {
    proxy_port: u16,
    proxy_addr: String,
    _temp_dir: TempDir,
    roxy_ca: RoxyCA,
    proxy_handle: TokioJoinHandle<()>,
}

impl Drop for TestContext {
    fn drop(&mut self) {
        self.proxy_handle.abort();
    }
}

impl TestContext {
    pub async fn new() -> Self {
        TestContext::init(None).await
    }
    pub async fn new_with_script(script: String) -> Self {
        TestContext::init(Some(script)).await
    }

    pub async fn init(script: Option<String>) -> Self {
        init_logging();
        init_crypto();

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let script_engine = match script {
            Some(script) => {
                let file_path = temp_dir.path().join("test.lua");
                let mut file = File::create(file_path.clone()).await.unwrap();
                file.write_all(script.as_bytes()).await.unwrap();
                Some(interceptor::ScriptEngine::new(file_path).unwrap())
            }
            None => None,
        };

        let proxy_port = rnd_ephemeral().await;
        let proxy_addr = format!("127.0.0.1:{proxy_port}");

        let flow_store = FlowStore::new();
        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();

        let proxy_handle = tokio::spawn(async move {
            start_proxy(proxy_port, roxy_ca, script_engine, flow_store).unwrap();
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();
        TestContext {
            proxy_port,
            proxy_addr,
            _temp_dir: temp_dir,
            proxy_handle,
            roxy_ca,
        }
    }
}

async fn rnd_ephemeral() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // TODO: is this automatically dropped
    port
}

async fn default_https() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let root = warp::path::end().map(|| {
        warp::http::Response::builder()
            .header("test", "test")
            .body("Hello from warp HTTPS test server")
            .unwrap()
    });

    let routes = warp::get().and(root);
    start_https_warp_server(routes).await
}

async fn default_http() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let root = warp::path::end().map(|| {
        warp::http::Response::builder()
            .header("test", "test")
            .body("Hello from warp test server")
            .unwrap()
    });

    let routes = warp::get().and(root);
    start_warp_test_server(routes).await
}

#[tokio::test]
async fn test_http_proxy_request() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_http().await;
    let target_url = format!("http://{server_addr}");

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert!(status.is_success());
    assert_eq!(body, "Hello from warp test server");

    server_handle.abort();
}

#[tokio::test]
async fn test_http_proxy_request_100() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_http().await;
    let target_url = format!("http://{server_addr}");

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    for _i in 0..100 {
        let res = client.get(&target_url).send().await.unwrap();
        let status = res.status();
        let body = res.text().await.unwrap();

        assert!(status.is_success());
        assert_eq!(body, "Hello from warp test server");
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_https_proxy_request() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .danger_accept_invalid_certs(true) // TODO: set the roxy cert here
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, 200);
    assert_eq!(body, "Hello from warp HTTPS test server");
    server_handle.abort();
}

#[tokio::test]
async fn test_https_proxy_request_100() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .danger_accept_invalid_certs(true) // TODO: set the roxy cert here
        .build()
        .unwrap();

    for _i in 0..100 {
        let res = client.get(&target_url).send().await.unwrap();
        let status = res.status();
        let body = res.text().await.unwrap();

        assert_eq!(status, 200);
        assert_eq!(body, "Hello from warp HTTPS test server");
    }
    server_handle.abort();
}

#[tokio::test]
async fn test_https2_proxy_request() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .http2_prior_knowledge()
        .danger_accept_invalid_certs(true) // TODO: set the roxy cert here
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, 200);
    assert_eq!(body, "Hello from warp HTTPS test server");
    server_handle.abort();
}

#[tokio::test]
async fn test_https2_proxy_request_100() {
    let cxt = TestContext::new().await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .http2_prior_knowledge()
        .danger_accept_invalid_certs(true) // TODO: set the roxy cert here
        .build()
        .unwrap();

    for _i in 0..100 {
        let res = client.get(&target_url).send().await.unwrap();
        let status = res.status();
        let body = res.text().await.unwrap();

        assert_eq!(status, 200);
        assert_eq!(body, "Hello from warp HTTPS test server");
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_rewrite_http_response_body() {
    let script = r#"
function intercept_request(req)
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    res.body = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let (server_addr, server_handle) = default_http().await;
    let target_url = format!("http://localhost:{}", server_addr.port());

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    );
    server_handle.abort();
}

#[tokio::test]
async fn test_rewrite_https_body() {
    let script = r#"
function intercept_request(req)
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    res.body = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http1_only()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    );
    server_handle.abort();
}

#[tokio::test]
async fn test_rewrite_http2_body() {
    let script = r#"
function intercept_request(req)
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    res.body = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let (server_addr, server_handle) = default_https().await;
    let target_url = format!("https://localhost:{}", server_addr.port());

    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .use_rustls_tls() // This is required for to rust our certs
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    );
    server_handle.abort();
}

#[tokio::test]
async fn test_redirect_http_request() {
    let (server_addr, server_handle) = default_http().await;

    let script = format!(
        r#"
function intercept_request(req)
    req.host = "localhost"
    req.port = {}
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        server_addr.port()
    );
    let cxt = TestContext::new_with_script(script).await;

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client
        .get("http://asdfasdfasdfasdf.com")
        .send()
        .await
        .unwrap();

    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello from warp test server");

    server_handle.abort();
}

#[tokio::test]
async fn test_redirect_https_request() {
    let (server_addr, server_handle) = default_https().await;

    let script = format!(
        r#"
function intercept_request(req)
    req.host = "localhost"
    req.port = {}
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        server_addr.port()
    );
    let cxt = TestContext::new_with_script(script).await;

    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    // be a roxy cert
    let client = reqwest::Client::builder()
        .http1_only()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client
        .get("https://asdfasdfasdfasdf.com")
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello from warp HTTPS test server");

    server_handle.abort();
}

#[tokio::test]
async fn test_redirect_https_request_to_http() {
    let (server_addr, server_handle) = default_http().await;

    let script = format!(
        r#"
function intercept_request(req)
    req.scheme = "http"
    req.port = {}
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        server_addr.port()
    );

    let cxt = TestContext::new_with_script(script).await;
    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http1_only()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let fut = client.get("https://localhost").send();
    match timeout(Duration::from_secs(5), fut).await.unwrap() {
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap();

            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, "Hello from warp test server");
            server_handle.abort();
        }
        Err(err) => {
            server_handle.abort();
            panic!("{}", err);
        }
    }
}

#[tokio::test]
async fn test_redirect_http_request_to_https() {
    let (server_addr, server_handle) = default_https().await;

    let script = format!(
        r#"
function intercept_request(req)
    req.scheme = "https"
    req.port = {}
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        server_addr.port()
    );
    let cxt = TestContext::new_with_script(script).await;

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let fut = client.get("http://localhost").send();
    match timeout(Duration::from_secs(5), fut).await.unwrap() {
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap();

            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, "Hello from warp HTTPS test server");
            server_handle.abort();
        }
        Err(err) => {
            server_handle.abort();
            panic!("{}", err);
        }
    }
}

#[tokio::test]
async fn test_redirect_http2_request() {
    let (server_addr, server_handle) = default_https().await;

    let script = format!(
        r#"
function intercept_request(req)
    req.host = "localhost"
    req.port = {}
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        server_addr.port()
    );
    let cxt = TestContext::new_with_script(script).await;

    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let res = client
        .get("https://asdfasdfasdfasdf.com")
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello from warp HTTPS test server");

    server_handle.abort();
}

#[tokio::test]
async fn test_change_path_http_request() {
    let (server_addr, server_handle) = default_http().await;

    let script = r#"
function intercept_request(req)
    req.path = ""
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("http://{server_addr}/missing");
    let res = client.get(target_url).send().await.unwrap();

    let status = res.status();
    let body = res.text().await.unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello from warp test server");

    server_handle.abort();
}

#[tokio::test]
async fn test_change_path_https_request() {
    let (server_addr, server_handle) = default_https().await;

    let script = r#"
function intercept_request(req)
    req.path = "/"
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#;

    let cxt = TestContext::new_with_script(script.to_string()).await;
    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http1_only()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("https://{server_addr}/missing");
    let fut = client.get(target_url).send();

    match timeout(Duration::from_secs(5), fut).await.unwrap() {
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap();

            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, "Hello from warp HTTPS test server");
            server_handle.abort();
        }
        Err(err) => {
            server_handle.abort();
            panic!("{}", err);
        }
    }
}

#[tokio::test]
async fn test_add_header_http_request() {
    let (server_addr, server_handle) = default_http().await;

    let script = r#"
function intercept_request(req)
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    res.headers['hi'] = 'there'
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("http://{server_addr}");
    let res = client.get(target_url).send().await.unwrap();

    let status = res.status();

    assert_eq!(status, StatusCode::OK);
    let header_v = res.headers().get("hi").unwrap();
    assert_eq!(header_v, "there");

    server_handle.abort();
}

#[tokio::test]
async fn test_add_header_https_request() {
    let (server_addr, server_handle) = default_https().await;

    let script = r#"
function intercept_request(req)
    req.path = "/"
    return req
end

function intercept_response(res)
    res.headers['hi'] = 'there'
    return res
end
"#;

    let cxt = TestContext::new_with_script(script.to_string()).await;
    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http1_only()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("https://{server_addr}/missing");
    let res = client.get(target_url).send().await.unwrap();

    let status = res.status();

    assert_eq!(status, StatusCode::OK);
    let header_v = res.headers().get("hi").unwrap();
    assert_eq!(header_v, "there");

    server_handle.abort();
}

#[tokio::test]
async fn test_add_header_http2_request() {
    let (server_addr, server_handle) = default_https().await;

    let script = r#"
function intercept_request(req)
    req.path = "/"
    return req
end

function intercept_response(res)
    res.headers['hi'] = 'there'
    return res
end
"#;

    let cxt = TestContext::new_with_script(script.to_string()).await;
    let reqwest_cert = reqwest::Certificate::from_der(&cxt.roxy_ca.ca_der).unwrap();

    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .use_rustls_tls()
        .add_root_certificate(reqwest_cert)
        .proxy(reqwest::Proxy::https(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("https://{server_addr}/missing");
    let res = client.get(target_url).send().await.unwrap();

    let status = res.status();

    assert_eq!(status, StatusCode::OK);
    let header_v = res.headers().get("hi").unwrap();
    assert_eq!(header_v, "there");

    server_handle.abort();
}

#[tokio::test]
async fn test_remove_header_http_request() {
    let (server_addr, server_handle) = default_http().await;

    let script = r#"
function intercept_request(req)
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    res.headers['test'] = None
    return res
end
"#;
    let cxt = TestContext::new_with_script(script.to_string()).await;

    let client = reqwest::Client::builder()
        .http1_only()
        .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
        .build()
        .unwrap();

    let target_url = format!("http://{server_addr}");
    let res = client.get(target_url).send().await.unwrap();

    let status = res.status();

    assert_eq!(status, StatusCode::OK);
    let header_v = res.headers().get("test");
    assert_eq!(header_v, None);

    server_handle.abort();
}

// TODO: might have to write Hyper only http1 server
// #[tokio::test]
// async fn down_grade_http2_http1() {
//     let (server_addr, server_handle) = default_http().await;
//
//     let cxt = TestContext::new().await;
//
//     let client = reqwest::Client::builder()
//         .http2_prior_knowledge()
//         .proxy(reqwest::Proxy::http(&cxt.proxy_addr).unwrap())
//         .build()
//         .unwrap();
//
//     let target_url = format!("http://{}", server_addr);
//     let res = client.get(target_url).send().await.unwrap();
//
//     let status = res.status();
//
//     assert_eq!(status, StatusCode::OK);
//     let header_v = res.headers().get("test");
//     assert_eq!(header_v, None);
//
//     server_handle.abort();
// }

#[tokio::test]
async fn ws_test() {
    let cxt = TestContext::new().await;

    let port = rnd_ephemeral().await;
    let handle = start_ws_server(port);
    let ws_addr = format!("ws://127.0.0.1:{port}");
    let target_host = format!("127.0.0.1:{port}");

    let proxy_addr = &cxt.proxy_addr;

    let mut proxy_stream = TcpStream::connect(proxy_addr).await.unwrap();

    let connect_req = format!("CONNECT {target_host} HTTP/1.1\r\nHost: {target_host}\r\n\r\n");

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

    let port = rnd_ephemeral().await;
    let server_handle = start_wss_server(port);

    let ws_addr = format!("wss://127.0.0.1:{port}");
    let target_host = format!("127.0.0.1:{port}");

    let proxy_addr = &cxt.proxy_addr;

    let mut proxy_stream = TcpStream::connect(proxy_addr).await.unwrap();

    let connect_req = format!("CONNECT {target_host} HTTP/1.1\r\nHost: {target_host}\r\n\r\n");

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
        .with_custom_certificate_verifier(Arc::new(LoggingCertVerifier::new()))
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
async fn test_h3() {
    let cxt = TestContext::new().await;
    let port = rnd_ephemeral().await;
    let h3_handle = default_h3(port, &cxt.roxy_ca).await;

    let host_addr: Uri = format!("https://127.0.0.1:{port}").parse().unwrap();
    let proxy_addr: Uri = format!("https://127.0.0.1:{}", cxt.proxy_port)
        .parse()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let req = http::Request::builder()
        .method(Method::GET)
        .header(HOST, host_addr.authority().unwrap().to_string())
        .body(())
        .unwrap();

    let ca_der = cxt.roxy_ca.ca_der.clone();
    let resp = h3_with_proxy(proxy_addr, host_addr, ca_der, req)
        .await
        .unwrap();

    let body = resp.body().to_owned();
    let a = body.clone().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&a);
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body, "hello".to_string());

    h3_handle.abort();
    drop(cxt);
}
