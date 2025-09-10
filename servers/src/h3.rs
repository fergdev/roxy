use std::{
    error::Error,
    io,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
};

use bytes::{Buf, Bytes, BytesMut};
use h3::server::RequestResolver;
use http::Response;
use http_body_util::BodyExt;
use quinn::{EndpointConfig, crypto::rustls::QuicServerConfig, default_runtime};
use roxy_shared::{RoxyCA, alpn::alp_h3, io::local_udp_socket, tls::TlsConfig};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::serve::serve_internal;
use crate::{HttpServers, local_tls_config};

pub async fn h3_server(
    server: HttpServers,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let udp_socket = local_udp_socket(None)?;
    h3_server_socket(udp_socket, roxy_ca, server, tls_config).await
}

pub async fn h3_server_socket(
    udp_socket: UdpSocket,
    roxy_ca: &RoxyCA,
    server: HttpServers,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = udp_socket.local_addr()?;
    let server_crypto = local_tls_config(roxy_ca, tls_config, alp_h3())?;
    let server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));

    let runtime = default_runtime().ok_or_else(|| io::Error::other("no async runtime found"))?;
    let socket = runtime.wrap_udp_socket(udp_socket)?;
    let endpoint = quinn::Endpoint::new_with_abstract_socket(
        EndpointConfig::default(),
        Some(server_config),
        socket,
        runtime,
    )?;

    let handle = tokio::spawn(async move {
        info!("{server} server awaiting connections {addr}");
        while let Some(new_conn) = endpoint.accept().await {
            info!("New connection being attempted");

            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        info!("new connection established");
                        let mut h3_conn = match h3::server::Connection::new(
                            h3_quinn::Connection::new(conn),
                        )
                        .await
                        {
                            Ok(c) => c,
                            Err(e) => {
                                error!("Error {e}");
                                return;
                            }
                        };

                        loop {
                            match h3_conn.accept().await {
                                Ok(Some(resolver)) => {
                                    tokio::spawn(async move {
                                        if let Err(e) = handle_request(resolver, server).await {
                                            error!("handling request failed: {}", e);
                                        }
                                    });
                                }
                                Ok(None) => {
                                    error!("None on accept");
                                    break;
                                }
                                Err(err) => {
                                    warn!("error on accept {}", err);
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("accepting connection failed: {:?}", err);
                    }
                }
                info!("Connection closed");
            });
        }
        warn!("{server} stopped");
    });

    Ok((addr, handle))
}

async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
    server: HttpServers,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream) = resolver.resolve_request().await?;
    let (parts, _) = req.into_parts();
    let mut buf = BytesMut::new();
    while let Some(chunk) = stream.recv_data().await? {
        buf.extend_from_slice(chunk.chunk());
    }
    let body = buf.freeze();
    let trailers = stream.recv_trailers().await?;

    let resp = serve_internal(parts, body, trailers, server).await?;

    info!("Resp: {server} {resp:?}");
    let (parts, mut body) = resp.into_parts();

    let resp = Response::from_parts(parts, ());
    stream.send_response(resp).await?;

    while let Some(Ok(a)) = body.frame().await {
        if let Some(data) = a.data_ref() {
            stream.send_data(data.clone()).await?;
        } else if let Ok(trailer) = a.into_trailers() {
            stream.send_trailers(trailer).await?;
        }
    }

    stream.finish().await?;
    Ok(())
}
