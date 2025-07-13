use futures_util::{SinkExt, StreamExt};
use rcgen::{CertifiedKey, generate_simple_self_signed};
use rustls::{ServerConfig, pki_types::PrivateKeyDer};
use std::sync::Arc;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};
use tracing::info;

pub fn start_ws_server(port: u16) -> JoinHandle<()> {
    let addr = format!("127.0.0.1:{port}");
    info!("Starting ws at {}", addr);
    tokio::spawn(async {
        let listener = TcpListener::bind(addr).await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws_stream = accept_async(stream).await.expect("WS handshake failed");
                handle_ws(ws_stream).await;
            });
        }
    })
}

pub fn start_wss_server(port: u16) -> JoinHandle<()> {
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let tls_config = {
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.der().clone()],
                PrivateKeyDer::try_from(signing_key.serialize_der()).unwrap(),
            )
            .expect("bad certs");

        TlsAcceptor::from(Arc::new(config))
    };

    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(addr).await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            let tls_acceptor = tls_config.clone();
            tokio::spawn(async move {
                match tls_acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let ws_stream = accept_async(tls_stream)
                            .await
                            .expect("WSS handshake failed");
                        handle_ws(ws_stream).await;
                    }
                    Err(err) => eprintln!("TLS error: {err:?}"),
                }
            });
        }
    })
}

async fn handle_ws<S>(ws: WebSocketStream<S>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
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
