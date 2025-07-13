use rcgen::{CertifiedKey, generate_simple_self_signed};
use std::time::Duration;
use tokio::time::timeout;
use warp::{Filter, reply::Reply};

pub mod h3;
pub mod ws;

pub async fn start_warp_test_server<F>(
    filter: F,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>)
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move {
        let server = warp::serve(filter);
        let (addr, fut) = server.bind_ephemeral(([127, 0, 0, 1], 0));
        addr_tx.send(addr).unwrap();
        fut.await;
    });

    (addr_rx.await.unwrap(), handle)
}

pub async fn start_https_warp_server<F>(
    filter: F,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>)
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();

    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();

    let handle = tokio::spawn(async move {
        let server = warp::serve(filter)
            .tls()
            .key(signing_key.serialize_pem())
            .cert(cert.pem());

        let (addr, fut) = server.bind_ephemeral(([127, 0, 0, 1], 0));
        addr_tx.send(addr).unwrap();
        timeout(Duration::from_secs(5), fut).await.unwrap(); // TODO: what is happening here
    });

    (addr_rx.await.unwrap(), handle)
}
