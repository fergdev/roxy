use std::{
    net::{SocketAddr, UdpSocket},
    process::Command,
    time::Duration,
};

use criterion::{Criterion, criterion_group, criterion_main};
use http::{Method, Version};
use http_body_util::{Empty, combinators::BoxBody};
use once_cell::sync::OnceCell;
use roxy_proxy::{flow::FlowStore, interceptor, proxy::ProxyManager};
use roxy_servers::{H11_BODY, h1::h1_server};
use roxy_shared::{
    RoxyCA, client::ClientContext, crypto::init_crypto, generate_roxy_root_ca_with_path,
    http::HttpResponse, tls::TlsConfig, uri::RUri,
};
use tempfile::TempDir;
use tokio::{
    net::TcpListener,
    time::{sleep, timeout},
};

pub static INIT_LOGGER: OnceCell<()> = OnceCell::new();

pub fn init_logging() {
    INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .init();
    });
}

pub struct TestContext {
    _proxy_socket_addr: SocketAddr,
    proxy_addr: RUri,
    _temp_dir: TempDir,
    roxy_ca: RoxyCA,
    _proxy_manager: ProxyManager,
}

impl TestContext {
    pub async fn new() -> Self {
        init_logging();
        init_crypto();

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let script_engine = interceptor::ScriptEngine::new().await.unwrap();

        let flow_store = FlowStore::new();
        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_socket_addr = listener.local_addr().unwrap();
        let proxy_port = proxy_socket_addr.port();
        let addr = format!("127.0.0.1:{proxy_port}");
        let proxy_uri: RUri = addr.parse().unwrap();
        let udp_socket = UdpSocket::bind(format!("127.0.0.1:{proxy_port}")).unwrap();

        let tls_config = TlsConfig::default();
        let mut proxy_manager =
            ProxyManager::new(0, roxy_ca, script_engine, tls_config, flow_store);
        proxy_manager.start_tcp(listener).await.unwrap();
        proxy_manager.start_udp(udp_socket).await.unwrap();

        sleep(Duration::from_millis(300)).await;

        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();
        TestContext {
            _proxy_socket_addr: proxy_socket_addr,
            proxy_addr: proxy_uri,
            _temp_dir: temp_dir,
            _proxy_manager: proxy_manager,
            roxy_ca,
        }
    }
}

fn criterion_benchmark_roxy(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let cxt = TestContext::new().await;

        let (server_addr, server_handle) = h1_server(roxy_servers::HttpServers::H11).await.unwrap();

        let target_uri: RUri = format!("http://{server_addr}").parse().unwrap();
        c.bench_function("http get roxy", |b| {
            b.iter(|| async {
                let req = http::Request::builder()
                    .method(Method::GET)
                    .version(Version::HTTP_11)
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
                } = timeout(Duration::from_millis(300), client.request(req))
                    .await
                    .unwrap()
                    .unwrap();

                assert!(trailers.is_none());
                assert_eq!(parts.status, 200);
                assert_eq!(body, H11_BODY);
            });
        });
        server_handle.abort();
    });
}

fn criterion_benchmark_roxy_multi(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let cxt = TestContext::new().await;

        let (server_addr, server_handle) = h1_server(roxy_servers::HttpServers::H11).await.unwrap();

        c.bench_function("http get roxy multi", |b| {
            b.iter(|| async {
                let mut handles = vec![];
                let proxy_uri: RUri = cxt.proxy_addr.clone();
                let target_uri: RUri = format!("http://{server_addr}").parse().unwrap();
                for _i in 0..10000 {
                    let proxy_uri = proxy_uri.clone();
                    let target_uri = target_uri.clone();
                    let ca = cxt.roxy_ca.clone();
                    let handle = tokio::spawn(async move {
                        let req = http::Request::builder()
                            .method(Method::GET)
                            .version(Version::HTTP_11)
                            .uri(target_uri.clone())
                            .body(BoxBody::new(Empty::new()))
                            .unwrap();

                        let client = ClientContext::builder()
                            .with_proxy(proxy_uri)
                            .with_roxy_ca(ca)
                            .build();
                        let HttpResponse {
                            parts,
                            body,
                            trailers,
                        } = timeout(Duration::from_millis(300), client.request(req))
                            .await
                            .unwrap()
                            .unwrap();

                        assert!(trailers.is_none());
                        assert_eq!(parts.status, 200);
                        assert_eq!(body, H11_BODY);
                    });

                    handles.push(handle);
                }
                for h in handles {
                    let _ = tokio::join!(h);
                }
            });
        });
        server_handle.abort();
    });
}

#[allow(clippy::zombie_processes)]
fn criterion_benchmark_mitm(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();
        let mut ca_path = temp_dir.path().to_path_buf();
        ca_path.push("roxy-ca.pem");
        let proxy_port = 6161;

        let mut a = Command::new("mitmproxy")
            .arg("--mode")
            .arg("regular")
            .arg("--listen-port")
            .arg(format!("{proxy_port}"))
            .arg("--certs")
            .arg(ca_path.to_str().unwrap())
            .spawn()
            .expect("can't execute");

        let (server_addr, server_handle) = h1_server(roxy_servers::HttpServers::H11).await.unwrap();

        let proxy_uri: RUri = format!("http://localhost:{proxy_port}").parse().unwrap();
        let target_uri: RUri = format!("http://{server_addr}").parse().unwrap();

        c.bench_function("http get mitm", |b| {
            b.iter(|| async {
                let req = http::Request::builder()
                    .method(Method::GET)
                    .version(Version::HTTP_11)
                    .uri(target_uri.clone())
                    .body(BoxBody::new(Empty::new()))
                    .unwrap();

                let client = ClientContext::builder()
                    .with_proxy(proxy_uri.clone())
                    .with_roxy_ca(roxy_ca.clone())
                    .build();

                let HttpResponse {
                    parts,
                    body,
                    trailers,
                } = timeout(Duration::from_millis(300), client.request(req))
                    .await
                    .unwrap()
                    .unwrap();

                assert!(trailers.is_none());
                assert_eq!(parts.status, 200);
                assert_eq!(body, H11_BODY);
            })
        });
        a.kill().unwrap();
        server_handle.abort();
        drop(roxy_ca);
    });
}

#[allow(clippy::zombie_processes)]
fn criterion_benchmark_mitm_multi(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();
        let mut ca_path = temp_dir.path().to_path_buf();
        ca_path.push("roxy-ca.pem");
        let proxy_port = 6161;

        let mut a = Command::new("mitmproxy")
            .arg("--mode")
            .arg("regular")
            .arg("--listen-port")
            .arg(format!("{proxy_port}"))
            .arg("--certs")
            .arg(ca_path.to_str().unwrap())
            .spawn()
            .expect("can't execute");

        let (server_addr, server_handle) = h1_server(roxy_servers::HttpServers::H11).await.unwrap();

        c.bench_function("http get mitm multi", |b| {
            b.iter(|| async {
                let mut handles = vec![];
                let proxy_uri: RUri = format!("http://localhost:{proxy_port}").parse().unwrap();
                let target_uri: RUri = format!("http://{server_addr}").parse().unwrap();
                for _i in 0..10000 {
                    let proxy_uri = proxy_uri.clone();
                    let target_uri = target_uri.clone();
                    let roxy_ca = roxy_ca.clone();
                    let handle = tokio::spawn(async move {
                        let req = http::Request::builder()
                            .method(Method::GET)
                            .version(Version::HTTP_11)
                            .uri(target_uri.clone())
                            .body(BoxBody::new(Empty::new()))
                            .unwrap();

                        let client = ClientContext::builder()
                            .with_proxy(proxy_uri.clone())
                            .with_roxy_ca(roxy_ca.clone())
                            .build();
                        let HttpResponse {
                            parts,
                            body,
                            trailers,
                        } = timeout(Duration::from_millis(300), client.request(req))
                            .await
                            .unwrap()
                            .unwrap();

                        assert!(trailers.is_none());
                        assert_eq!(parts.status, 200);
                        assert_eq!(body, H11_BODY);
                    });

                    handles.push(handle);
                }
                for h in handles {
                    let _ = tokio::join!(h);
                }
            })
        });
        a.kill().unwrap();
        server_handle.abort();
        drop(roxy_ca);
    });
}

criterion_group!(
    benches,
    criterion_benchmark_roxy,
    criterion_benchmark_roxy_multi,
    criterion_benchmark_mitm,
    criterion_benchmark_mitm_multi
);
criterion_main!(benches);
