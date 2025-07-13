use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use h3::server::RequestResolver;
use quinn::crypto::rustls::QuicServerConfig;
use roxy_shared::RoxyCA;
use rustls::{ServerConfig as RustlsServerConfig, pki_types::PrivateKeyDer};
use tokio::task::JoinHandle;
use tracing::{error, info};

pub async fn default_h3(port: u16, roxy_ca: &RoxyCA) -> JoinHandle<()> {
    let (cert, signing_key) = roxy_ca
        .sign_leaf_mult(
            "localhost",
            vec!["localhost".to_string(), "127.0.0.1".to_string()],
        )
        .unwrap();
    tokio::spawn(async move {
        info!("H3 server Spawn {port}");
        let mut server_crypto = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.der().clone()],
                PrivateKeyDer::try_from(signing_key.serialize_der()).unwrap(),
            )
            .unwrap();

        server_crypto.alpn_protocols = vec![b"h3".to_vec()];

        let opt = SocketAddr::from(([127, 0, 0, 1], port));
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto).unwrap(),
        ));
        let endpoint = quinn::Endpoint::server(server_config, opt).unwrap();

        info!("H3 server awaiting connections");
        while let Some(new_conn) = endpoint.accept().await {
            info!("New connection being attempted");

            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        info!("new connection established");

                        let mut h3_conn =
                            h3::server::Connection::new(h3_quinn::Connection::new(conn))
                                .await
                                .unwrap();

                        loop {
                            match h3_conn.accept().await {
                                Ok(Some(resolver)) => {
                                    tokio::spawn(async {
                                        if let Err(e) = handle_request(resolver).await {
                                            error!("handling request failed: {}", e);
                                        }
                                    });
                                }
                                Ok(None) => {
                                    error!("None on accept");
                                    break;
                                }
                                Err(err) => {
                                    error!("error on accept {}", err);
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("accepting connection failed: {:?}", err);
                    }
                }
            });
        }
        error!("HTTP/3 server stopped accepting connections");
    })
}

async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream) = resolver.resolve_request().await?;
    let path = req.uri().path();

    info!("H3 Server path {path}");
    if path == "/" {
        let response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header("content-type", "text/plain")
            .body(())
            .unwrap();

        stream.send_response(response).await?;
        stream.send_data(Bytes::from_static(b"hello")).await?;
    } else {
        let response = http::Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header("content-type", "text/plain")
            .body(())
            .unwrap();

        stream.send_response(response).await?;
    }
    Ok(())
}
