use std::{io::Error, sync::Arc};

use futures::{SinkExt, StreamExt};
use roxy_shared::tls::RustlsClientConfig;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    Connector, WebSocketStream, accept_async, connect_async_tls_with_config,
    tungstenite::client::IntoClientRequest,
};
use tracing::info;

use crate::{
    flow::{FlowConnection, FlowEvent, WsMessage},
    proxy::FlowContext,
};

pub async fn handle_ws<S>(
    flow_cxt: FlowContext,
    stream: S,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    info!("Handing WS {:?}", flow_cxt.target_uri);

    let flow_id = flow_cxt
        .proxy_cxt
        .flow_store
        .new_ws_flow(FlowConnection {
            addr: flow_cxt.client_addr,
        })
        .await;

    info!("Client accept");
    let ws_client = accept_async(stream).await.map_err(Error::other)?;
    let server_stream = TcpStream::connect(&flow_cxt.target_uri.host_port()).await?;

    info!("ws server connect");
    let ws_server = tokio_tungstenite::client_async("ws://fake", server_stream)
        .await
        .map(|(ws, _resp)| ws)
        .map_err(Error::other)?;

    process_ws(flow_id, flow_cxt, ws_client, ws_server).await?;
    Ok(())
}

pub async fn handle_wss<S>(
    flow_cxt: FlowContext,
    stream: S,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let flow_id = flow_cxt
        .proxy_cxt
        .flow_store
        .new_ws_flow(FlowConnection {
            addr: flow_cxt.client_addr,
        })
        .await;

    let ws_client = accept_async(stream).await.map_err(Error::other)?;

    let RustlsClientConfig {
        cert_logger: _,
        resolver: _,
        client_config,
    } = flow_cxt
        .proxy_cxt
        .tls_config
        .rustls_client_config(flow_cxt.proxy_cxt.ca.roots());

    let url = format!("wss://{}", flow_cxt.target_uri);
    let req = url.clone().into_client_request().map_err(Error::other)?;

    let (ws_server, _) = connect_async_tls_with_config(
        req,
        None,
        false,
        Some(Connector::Rustls(Arc::new(client_config))),
    )
    .await
    .map_err(Error::other)?;

    process_ws(flow_id, flow_cxt, ws_client, ws_server).await?;
    Ok(())
}

async fn process_ws<S, T>(
    flow_id: i64,
    flow_cxt: FlowContext,
    ws_client: WebSocketStream<S>,
    ws_server: WebSocketStream<T>,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut client_write, mut client_read) = ws_client.split();
    let (mut server_write, mut server_read) = ws_server.split();

    let client_to_server = async {
        while let Some(msg) = client_read.next().await {
            let msg = msg.map_err(Error::other)?;
            flow_cxt.proxy_cxt.flow_store.post_event(
                flow_id,
                FlowEvent::WsMessage(WsMessage::client(msg.clone())),
            );
            server_write.send(msg).await.map_err(Error::other)?;
        }
        Ok::<_, Error>(())
    };

    let server_to_client = async {
        while let Some(msg) = server_read.next().await {
            let msg = msg.map_err(Error::other)?;
            flow_cxt.proxy_cxt.flow_store.post_event(
                flow_id,
                FlowEvent::WsMessage(WsMessage::server(msg.clone())),
            );
            client_write.send(msg).await.map_err(Error::other)?;
        }
        Ok::<_, Error>(())
    };

    tokio::select! {
        res = client_to_server => res,
        res = server_to_client => res,
    }
    .map_err(Box::new)?;
    Ok(())
}
