#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use roxy_servers::{
    HttpServers,
    h1::h1_server_listener,
    h2::h2_server_listener,
    h3::h3_server_socket,
    ws::{start_ws_server, start_wss_server},
};
use roxy_shared::io::local_tcp_listener;
use roxy_shared::io::local_udp_socket;
use roxy_shared::{generate_roxy_root_ca, tls::TlsConfig};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let certs = generate_roxy_root_ca()?;

    let tls_config = TlsConfig::default();

    let (_, http_server) = h1_server_listener(
        local_tcp_listener(Some(8000)).await?,
        roxy_servers::HttpServers::H11,
    )
    .await?;
    let (_, https_server) = h1_server_listener(
        local_tcp_listener(Some(8001)).await?,
        roxy_servers::HttpServers::H11,
    )
    .await?;
    let (_, http2_server) = h2_server_listener(
        local_tcp_listener(Some(8002)).await?,
        HttpServers::H2,
        &certs,
        &tls_config,
    )
    .await?;
    let (_, http2_h1_server) = h2_server_listener(
        local_tcp_listener(Some(8003)).await?,
        HttpServers::H2,
        &certs,
        &tls_config,
    )
    .await?;
    let (_, http3_server) = h3_server_socket(
        local_udp_socket(Some(8004))?,
        &certs,
        HttpServers::H3,
        &tls_config,
    )
    .await?;
    let ws_server = start_ws_server(local_tcp_listener(Some(8005)).await?).await?;
    let wss_server =
        start_wss_server(local_tcp_listener(Some(8006)).await?, &certs, &tls_config).await?;

    println!("HTTP    →   http://localhost:8000");
    println!("HTTPS   →   http://localhost:8001");
    println!("HTTP2   →   https://localhost:8002");
    println!("HTTPS/2 →   https://localhost:8003");
    println!("HTTP3   →   https://localhost:8004");
    println!("WS      →   ws://localhost:8005");
    println!("WSS     →   wss://localhost:8006");

    let _ = tokio::join!(
        http_server,
        https_server,
        http2_server,
        http2_h1_server,
        http3_server,
        ws_server,
        wss_server
    );
    Ok(())
}
