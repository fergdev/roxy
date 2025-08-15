use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use roxy_shared::{
    RoxyCA,
    alpn::{alp_h1, alp_h2},
    tls::TlsConfig,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};
use tracing::info;

use crate::local_tls_acceptor;

pub async fn start_ws_server(tcp_listener: TcpListener) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let addr = tcp_listener.local_addr()?;
    info!("Starting ws at {}", addr);
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = tcp_listener.accept().await {
            tokio::spawn(async move {
                if let Ok(ws_stream) = accept_async(stream).await {
                    handle_ws(ws_stream).await;
                }
            });
        }
    });
    Ok(handle)
}

pub async fn start_wss_server(
    tcp_listener: TcpListener,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let tls_acceptor = local_tls_acceptor(roxy_ca, tls_config, alp_h1())?;
    let addr = tcp_listener.local_addr()?;
    info!("Starting ws at {}", addr);

    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = tcp_listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            tokio::spawn(async move {
                match tls_acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        if let Ok(ws_stream) = accept_async(tls_stream).await {
                            handle_ws(ws_stream).await;
                        }
                    }
                    Err(err) => eprintln!("TLS error: {err:?}"),
                }
            });
        }
    });

    Ok(handle)
}
pub async fn start_wss_h2_server(
    tcp_listener: TcpListener,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let tls_acceptor = local_tls_acceptor(roxy_ca, tls_config, alp_h2())?;
    let addr = tcp_listener.local_addr()?;
    info!("Starting ws at {}", addr);

    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = tcp_listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            tokio::spawn(async move {
                match tls_acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        if let Ok(ws_stream) = accept_async(tls_stream).await {
                            handle_ws(ws_stream).await;
                        }
                    }
                    Err(err) => eprintln!("TLS error: {err:?}"),
                }
            });
        }
    });

    Ok(handle)
}

async fn handle_ws<S>(ws: WebSocketStream<S>)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("New WebSocket connection");
    let (mut write, mut read) = ws.split();
    while let Some(Ok(msg)) = read.next().await {
        info!("Received: {:?}", msg);
        if msg.is_text() || msg.is_binary() {
            let _ = write.send(Message::Text("hello".into())).await;
        }
    }
}
