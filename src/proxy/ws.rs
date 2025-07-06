use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rustls::ClientConfig;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::RwLock,
};
use tokio_tungstenite::{
    Connector, accept_async, connect_async_tls_with_config, tungstenite::client::IntoClientRequest,
};

use crate::{
    flow::{Flow, FlowKind, FlowStore, WsFlow, WsMessage, WssFlow},
    notify_error,
    proxy::cert::LoggingCertVerifier,
};

pub async fn handle_ws<S>(
    stream: S,
    target_addr: &str,
    flow: Arc<RwLock<Flow>>,
    connect: crate::flow::InterceptedRequest,
    flow_store: FlowStore,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    flow.write().await.kind = FlowKind::Ws(WsFlow::new(connect));
    flow_store.notify();
    let ws_client = accept_async(stream)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let server_stream = tokio::net::TcpStream::connect(target_addr).await?;
    let ws_server = tokio_tungstenite::client_async("ws://fake", server_stream)
        .await
        .map(|(ws, _resp)| ws)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let (mut client_write, mut client_read) = ws_client.split();
    let (mut server_write, mut server_read) = ws_server.split();

    let to_server = async {
        while let Some(msg) = client_read.next().await {
            let msg = msg.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let mut guard = flow.write().await;
            match &mut guard.kind {
                FlowKind::Ws(flow) => {
                    flow.messages.push(WsMessage::client(msg.clone()));
                }
                _ => {
                    panic!("Unexpected flow kind");
                }
            }
            flow_store.notify();
            server_write
                .send(msg)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok::<(), std::io::Error>(())
    };

    let to_client = async {
        while let Some(msg) = server_read.next().await {
            let msg = msg.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let mut guard = flow.write().await;
            match &mut guard.kind {
                FlowKind::Ws(flow) => {
                    flow.messages.push(WsMessage::server(msg.clone()));
                }
                _ => {
                    panic!("Unexpected flow kind");
                }
            }
            flow_store.notify();
            client_write
                .send(msg)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok::<(), std::io::Error>(())
    };

    tokio::try_join!(to_server, to_client)?;
    Ok(())
}

pub async fn handle_wss<S>(
    stream: S,
    target_addr: &str,
    flow: Arc<RwLock<Flow>>,
    connect: crate::flow::InterceptedRequest,
    flow_store: FlowStore,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    flow.write().await.kind = FlowKind::Wss(WssFlow::new(connect));
    flow_store.notify();

    let ws_stream = accept_async(stream).await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("WS accept failed: {e}"))
    })?;

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LoggingCertVerifier::new()))
        .with_no_client_auth();

    let url = format!("wss://{}", target_addr);

    let req = url.clone().into_client_request().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("WS accept failed: {e}"))
    })?;
    let (server_ws_stream, _) =
        connect_async_tls_with_config(req, None, false, Some(Connector::Rustls(Arc::new(config))))
            .await
            .map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("WS connect failed: {e}"))
            })?;

    let (mut client_write, mut client_read) = ws_stream.split();
    let (mut server_write, mut server_read) = server_ws_stream.split();

    let client_to_server = async {
        while let Some(msg) = client_read.next().await {
            match msg {
                Ok(msg) => {
                    let mut guard = flow.write().await;
                    match &mut guard.kind {
                        FlowKind::Wss(flow) => {
                            flow.messages.push(WsMessage::client(msg.clone()));
                        }
                        _ => {
                            panic!("Unexpected flow kind");
                        }
                    }
                    flow_store.notify();
                    server_write.send(msg).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WS accept failed: {e}"),
                        )
                    })?;
                }
                Err(e) => {
                    // TODO: add to flow
                    notify_error!("WSS server read error: {}", e);
                    break;
                }
            }
        }
        Ok::<_, std::io::Error>(())
    };

    let server_to_client = async {
        while let Some(msg) = server_read.next().await {
            match msg {
                Ok(msg) => {
                    let mut guard = flow.write().await;
                    match &mut guard.kind {
                        FlowKind::Wss(flow) => {
                            flow.messages.push(WsMessage::client(msg.clone()));
                        }
                        _ => {
                            panic!("Unexpected flow kind");
                        }
                    }
                    flow_store.notify();
                    client_write.send(msg).await.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WS accept failed: {e}"),
                        )
                    })?;
                }
                Err(e) => {
                    // TODO: add to flow
                    notify_error!("WSS server read error: {}", e);
                    break;
                }
            }
        }
        Ok::<_, std::io::Error>(())
    };

    tokio::select! {
        res = client_to_server => res,
        res = server_to_client => res,
    }
}
