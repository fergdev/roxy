use std::time::Duration;

use rcgen::{CertifiedKey, generate_simple_self_signed};
use roxy::event::Event;
use roxy::flow::FlowStore;
use roxy::logging::initialize_logging;
use roxy::{interceptor, proxy};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;
use warp::Filter;

pub fn start_warp_test_server() -> std::net::SocketAddr {
    // Define route
    let route = warp::any().map(|| warp::reply::html("Hello from warp test server"));

    // Bind to random port
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let server = warp::serve(route);
            let (addr, fut) = server.bind_ephemeral(([127, 0, 0, 1], 0));
            addr_tx.send(addr).unwrap();
            fut.await;
        });
    });

    addr_rx.recv().unwrap()
}
pub fn start_https_warp_server() -> std::net::SocketAddr {
    let route = warp::any().map(|| warp::reply::html("Hello from warp HTTPS test server"));

    // Generate a self-signed cert for localhost
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();

    let (addr_tx, addr_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let server = warp::serve(route)
                .tls()
                .key(key_pair.serialize_pem())
                .cert(cert.pem());
            let (addr, fut) = server.bind_ephemeral(([127, 0, 0, 1], 0));
            addr_tx.send(addr).unwrap();
            fut.await;
        });
    });

    addr_rx.recv().unwrap()
}

#[tokio::test]
async fn test_http_proxy_request() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();

    let server_addr = start_warp_test_server();
    let target_url = format!("http://{}", server_addr);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let flow_store = FlowStore::new();

    let ca = roxy::certs::generate_roxy_root_ca_with_path(Some(temp_dir_path)).unwrap();
    let handle = tokio::spawn(async move {
        proxy::start_proxy(port, ca, None, flow_store).unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let proxy_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&proxy_url).unwrap())
        .build()
        .unwrap();

    println!("target_url URL: {}", target_url);
    println!("proxy URL: {}", proxy_url);
    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert!(status.is_success());
    assert_eq!(body, "Hello from warp test server");
    handle.abort();
}

#[tokio::test]
async fn test_https_proxy_request() {
    initialize_logging().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();

    let server_addr = start_https_warp_server();
    let target_url = format!("https://localhost:{}", server_addr.port());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let flow_store = FlowStore::new();
    let ca = roxy::certs::generate_roxy_root_ca_with_path(Some(temp_dir_path)).unwrap();
    let handle = tokio::spawn(async move {
        proxy::start_proxy(port, ca, None, flow_store).unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let proxy_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&proxy_url).unwrap())
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    println!("target_url URL: {}", target_url);
    println!("proxy URL: {}", proxy_url);
    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert!(status.is_success());
    assert_eq!(body, "Hello from warp HTTPS test server");
    handle.abort();
}

#[tokio::test]
async fn test_rewrite_https_proxy_request() {
    initialize_logging().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();

    let server_addr = start_https_warp_server();
    let target_url = format!("https://localhost:{}", server_addr.port());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (tx, mut _rx) = unbounded_channel::<Event>();

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
    let file_path = temp_dir.path().join("test.lua");
    let mut file = File::create(file_path.clone()).await.unwrap();
    file.write_all(script.as_bytes()).await.unwrap();

    let flow_store = FlowStore::new();

    let se = interceptor::ScriptEngine::new(file_path.to_str().unwrap().to_string()).unwrap();
    let ca = roxy::certs::generate_roxy_root_ca_with_path(Some(temp_dir_path)).unwrap();
    let handle = tokio::spawn(async move {
        proxy::start_proxy(port, ca, Some(se), flow_store).unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let proxy_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::https(&proxy_url).unwrap())
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    println!("target_url URL: {}", target_url);
    println!("proxy URL: {}", proxy_url);
    let res = client.get(&target_url).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert!(status.is_success());
    assert_eq!(
        body,
        "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"
    );
    handle.abort();
}

#[tokio::test]
async fn test_redirect_https_proxy_request() {
    initialize_logging().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();

    let server_addr = start_https_warp_server();
    let target_url = format!("https://localhost:{}", server_addr.port());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let script = format!(
        r#"
function intercept_request(req)
    req.uri = "{}"
    return req
end

function intercept_response(res)
    print("[lua] intercept_response")
    return res
end
"#,
        target_url
    );
    let file_path = temp_dir.path().join("test.lua");
    let mut file = File::create(file_path.clone()).await.unwrap();
    file.write_all(script.as_bytes()).await.unwrap();

    let flow_store = FlowStore::new();
    let se = interceptor::ScriptEngine::new(file_path.to_str().unwrap().to_string()).unwrap();
    let ca = roxy::certs::generate_roxy_root_ca_with_path(Some(temp_dir_path)).unwrap();
    let handle = tokio::spawn(async move {
        proxy::start_proxy(port, ca, Some(se), flow_store).unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let proxy_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::https(&proxy_url).unwrap())
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let rewrite_target = "https://asdfasdfasdfasdf.com";
    println!("target_url URL: {}", target_url);
    println!("proxy URL: {}", proxy_url);
    let res = client.get(rewrite_target).send().await.unwrap();
    let status = res.status();
    let body = res.text().await.unwrap();

    assert!(status.is_success());
    assert_eq!(body, "Hello from warp HTTPS test server");
    handle.abort();
}
