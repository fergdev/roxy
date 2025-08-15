use std::{error::Error, net::SocketAddr};

use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use roxy_shared::{
    RoxyCA,
    alpn::{alp_h1_h2, alp_h2},
    io::local_tcp_listener,
    tls::TlsConfig,
};
use tokio::{net::TcpListener, task::JoinHandle};
use tracing::{error, info, warn};

use crate::{HttpServers, local_tls_acceptor};
type H2ServerBuilder<TokioIo> = hyper::server::conn::http2::Builder<TokioIo>;

pub async fn h2_server(
    server: HttpServers,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    h2_server_listener(local_tcp_listener(None).await?, server, roxy_ca, tls_config).await
}

pub async fn h2_server_listener(
    tcp_listener: TcpListener,
    server: HttpServers,
    roxy_ca: &RoxyCA,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = tcp_listener.local_addr()?;
    let acceptor = local_tls_acceptor(roxy_ca, tls_config, alp_h2())?;
    info!("{server} listening on {addr}");

    let h = tokio::spawn(async move {
        info!("{server} listening on {}", addr);
        while let Ok((stream, _addr)) = tcp_listener.accept().await {
            info!("Creating TLS acceptor for client stream");
            if let Ok(client_tls) = acceptor.accept(stream).await {
                info!("{server} accepting request from {_addr}");
                tokio::task::spawn(async move {
                    if let Err(err) = H2ServerBuilder::new(TokioExecutor::new())
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

    Ok((addr, h))
}
pub async fn h2_h1_server(
    roxy_ca: &RoxyCA,
    server: HttpServers,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let tcp_listener = local_tcp_listener(None).await?;
    h2_h1_server_listener(tcp_listener, roxy_ca, server, tls_config).await
}

pub async fn h2_h1_server_listener(
    tcp_listener: TcpListener,
    roxy_ca: &RoxyCA,
    server: HttpServers,
    tls_config: &TlsConfig,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = tcp_listener.local_addr()?;

    let acceptor = local_tls_acceptor(roxy_ca, tls_config, alp_h1_h2())?;
    let h = tokio::spawn(async move {
        info!("{server} listening on {}", addr);
        while let Ok((stream, _addr)) = tcp_listener.accept().await {
            if let Ok(client_tls) = acceptor.accept(stream).await {
                info!("{server} accepting request from {_addr}");
                tokio::task::spawn(async move {
                    if let Err(err) = H2ServerBuilder::new(TokioExecutor::new())
                        .serve_connection(
                            TokioIo::new(client_tls),
                            service_fn(move |req| crate::serve::serve(req, server)),
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
    Ok((addr, h))
}
