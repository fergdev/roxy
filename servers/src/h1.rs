use std::{error::Error, net::SocketAddr};

use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use roxy_shared::{RoxyCA, alpn::alp_h1, io::local_tcp_listener, tls::TlsConfig};
use tokio::{net::TcpListener, task::JoinHandle};
use tracing::{error, info, warn};

use crate::{HttpServers, local_tls_acceptor};
type H1ServerBuilder = hyper::server::conn::http1::Builder;

pub async fn h1s_server(
    server: HttpServers,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    h1s_server_listener(server, local_tcp_listener(None).await?, roxy_ca, tls_config).await
}

pub async fn h1s_server_listener(
    server: HttpServers,
    tcp_listener: TcpListener,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = tcp_listener.local_addr()?;
    let acceptor = local_tls_acceptor(roxy_ca, tls_config, alp_h1())?;
    let handle = tokio::spawn(async move {
        info!("{server} listening on {}", addr);
        while let Ok((stream, _addr)) = tcp_listener.accept().await {
            info!("{server} request from {_addr}");
            if let Ok(client_tls) = acceptor.accept(stream).await {
                tokio::task::spawn(async move {
                    if let Err(err) = H1ServerBuilder::new()
                        .preserve_header_case(true)
                        .serve_connection(
                            TokioIo::new(client_tls),
                            service_fn(|req| crate::serve::serve(req, server)),
                        )
                        .await
                    {
                        error!("{server} server error: {err:?}");
                    }
                });
            }
        }
        warn!("{server} stopped");
    });

    Ok((addr, handle))
}

pub async fn h1_server(
    http_server: HttpServers,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    h1_server_listener(local_tcp_listener(None).await?, http_server).await
}

pub async fn h1_server_listener(
    tcp_listener: TcpListener,
    server: HttpServers,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = tcp_listener.local_addr()?;
    let handle = tokio::spawn(async move {
        info!("{server} listening on {}", addr);
        while let Ok((stream, _addr)) = tcp_listener.accept().await {
            info!("{server} Accepting request from {_addr}");
            tokio::task::spawn(async move {
                if let Err(err) = H1ServerBuilder::new()
                    .preserve_header_case(true)
                    .serve_connection(
                        TokioIo::new(stream),
                        service_fn(|req| crate::serve::serve(req, server)),
                    )
                    .await
                {
                    error!("{server} server error: {err:?}");
                }
            });
        }
        warn!("{server} stopped");
    });

    Ok((addr, handle))
}
