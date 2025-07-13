use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use once_cell::sync::OnceCell;
use roxy_proxy::{flow::FlowStore, interceptor, proxy::start_proxy};
use roxy_servers::start_warp_test_server;
use roxy_shared::{RoxyCA, generate_roxy_root_ca_with_path, init_crypto};
use tempfile::TempDir;
use tokio::{fs::File, io::AsyncWriteExt, net::TcpListener, task::JoinHandle as TokioJoinHandle};
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
    proxy_addr: String,
    _temp_dir: TempDir,
    _roxy_ca: RoxyCA,
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
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let roxy_ca = generate_roxy_root_ca_with_path(Some(temp_dir_path.clone())).unwrap();
        TestContext {
            proxy_addr,
            _temp_dir: temp_dir,
            proxy_handle,
            _roxy_ca: roxy_ca,
        }
    }
}

async fn rnd_ephemeral() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // TODO: is this automatically dropped
    port
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

fn criterion_benchmark_roxy(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let cxt = TestContext::new().await;

        let (server_addr, server_handle) = default_http().await;

        c.bench_function("http get roxy", |b| {
            b.iter(|| async {
                let target_url = format!("http://{server_addr}");
                let proxy_addr = cxt.proxy_addr.clone();
                let client = reqwest::Client::builder()
                    .proxy(reqwest::Proxy::http(proxy_addr).unwrap())
                    .build()
                    .unwrap();
                let res = client.get(target_url).send().await.unwrap();
                let status = res.status();
                let body = res.text().await.unwrap();

                assert!(status.is_success());
                assert_eq!(body, "Hello from warp test server");
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

        let (server_addr, server_handle) = default_http().await;

        c.bench_function("http get roxy multi", |b| {
            b.iter(|| async {
                let mut handles = vec![];
                for _i in 0..10000 {
                    let target_url = format!("http://{server_addr}");
                    let proxy_addr = cxt.proxy_addr.clone();
                    let handle = tokio::spawn(async {
                        let client = reqwest::Client::builder()
                            .proxy(reqwest::Proxy::http(proxy_addr).unwrap())
                            .build()
                            .unwrap();
                        let res = client.get(target_url).send().await.unwrap();
                        let status = res.status();
                        let body = res.text().await.unwrap();

                        assert!(status.is_success());
                        assert_eq!(body, "Hello from warp test server");
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
        let proxy_port = rnd_ephemeral().await;

        let mut a = Command::new("mitmproxy")
            .arg("--mode")
            .arg("regular")
            .arg("--listen-port")
            .arg(format!("{proxy_port}"))
            .arg("--certs")
            .arg(ca_path.to_str().unwrap())
            .spawn()
            .expect("can't execute");

        let (server_addr, server_handle) = default_http().await;
        let target_url = format!("http://{server_addr}");

        let proxy_addr = format!("localhost:{proxy_port}");
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::http(proxy_addr).unwrap())
            .build()
            .unwrap();

        c.bench_function("http get mitm", |b| {
            b.iter(|| async {
                let res = client.get(&target_url).send().await.unwrap();
                let status = res.status();
                let body = res.text().await.unwrap();

                assert!(status.is_success());
                assert_eq!(body, "Hello from warp test server");
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
        let proxy_port = rnd_ephemeral().await;

        let mut a = Command::new("mitmproxy")
            .arg("--mode")
            .arg("regular")
            .arg("--listen-port")
            .arg(format!("{proxy_port}"))
            .arg("--certs")
            .arg(ca_path.to_str().unwrap())
            .spawn()
            .expect("can't execute");

        let (server_addr, server_handle) = default_http().await;

        c.bench_function("http get mitm multi", |b| {
            b.iter(|| async {
                let mut handles = vec![];
                for _i in 0..10000 {
                    let proxy_addr = format!("localhost:{proxy_port}");
                    let target_url = format!("http://{server_addr}");
                    let handle = tokio::spawn(async {
                        let client = reqwest::Client::builder()
                            .proxy(reqwest::Proxy::http(proxy_addr).unwrap())
                            .build()
                            .unwrap();
                        let res = client.get(target_url).send().await.unwrap();
                        let status = res.status();
                        let body = res.text().await.unwrap();

                        assert!(status.is_success());
                        assert_eq!(body, "Hello from warp test server");
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
