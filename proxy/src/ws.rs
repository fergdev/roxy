use std::{
    io::{self, Error},
    net::SocketAddr,
    sync::Arc,
};

use futures::{SinkExt, StreamExt};
use rustls::ClientConfig;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
    Connector, accept_async, connect_async_tls_with_config, tungstenite::client::IntoClientRequest,
};
use tracing::error;

use crate::{
    cert::LoggingCertVerifier,
    flow::{FlowConnection, FlowKind, FlowStore, WsFlow, WsMessage, WssFlow},
};

pub async fn handle_ws<S>(
    socket_addr: SocketAddr,
    stream: S,
    target_addr: &str,
    flow_store: FlowStore,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    error!("Handing ws");
    let flow = flow_store
        .new_flow(FlowConnection { addr: socket_addr })
        .await;

    flow.write().await.kind = FlowKind::Ws(WsFlow::new());
    flow_store.notify();

    error!("Client accept");
    let ws_client = accept_async(stream).await.map_err(Error::other)?;

    error!("server tcp connect {}", target_addr);
    let server_stream = tokio::net::TcpStream::connect(target_addr).await?;

    error!("ws server connect");
    let ws_server = tokio_tungstenite::client_async("ws://fake", server_stream)
        .await
        .map(|(ws, _resp)| ws)
        .map_err(Error::other)?;

    error!("connected");
    let (mut client_write, mut client_read) = ws_client.split();
    let (mut server_write, mut server_read) = ws_server.split();

    let to_server = async {
        while let Some(msg) = client_read.next().await {
            let msg = msg.map_err(Error::other)?;
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
            server_write.send(msg).await.map_err(Error::other)?;
        }
        Ok::<(), Error>(())
    };

    let to_client = async {
        while let Some(msg) = server_read.next().await {
            let msg = msg.map_err(Error::other)?;
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
            client_write.send(msg).await.map_err(Error::other)?;
        }
        Ok::<(), Error>(())
    };

    tokio::try_join!(to_server, to_client)?;
    Ok(())
}

pub async fn handle_wss<S>(
    socket_addr: SocketAddr,
    stream: S,
    target_addr: &str,
    flow_store: FlowStore,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let flow = flow_store
        .new_flow(FlowConnection { addr: socket_addr })
        .await;

    flow.write().await.kind = FlowKind::Wss(WssFlow::new());
    flow_store.notify();

    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| Error::other(format!("WS accept failed: {e}")))?;

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LoggingCertVerifier::new()))
        .with_no_client_auth();

    let url = format!("wss://{target_addr}");

    let req = url
        .clone()
        .into_client_request()
        .map_err(|e| Error::other(format!("WS accept failed: {e}")))?;
    let (server_ws_stream, _) =
        connect_async_tls_with_config(req, None, false, Some(Connector::Rustls(Arc::new(config))))
            .await
            .map_err(|e| Error::other(format!("WS connect failed: {e}")))?;

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
                    server_write
                        .send(msg)
                        .await
                        .map_err(|e| Error::other(format!("WS accept failed: {e}")))?;
                }
                Err(e) => {
                    // TODO: add to flow
                    error!("WSS server read error: {}", e);
                    break;
                }
            }
        }
        Ok::<_, Error>(())
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
                    client_write
                        .send(msg)
                        .await
                        .map_err(|e| Error::other(format!("WS accept failed: {e}")))?;
                }
                Err(e) => {
                    // TODO: add to flow
                    error!("WSS server read error: {}", e);
                    break;
                }
            }
        }
        Ok::<_, Error>(())
    };

    tokio::select! {
        res = client_to_server => res,
        res = server_to_client => res,
    }
}
