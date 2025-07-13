use bytes::{Bytes, BytesMut};
use h3::server::RequestResolver;
use roxy_servers::ws::{start_ws_server, start_wss_server};
use roxy_shared::{generate_roxy_root_ca, init_crypto};
use rustls::pki_types::PrivateKeyDer;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{error, info};
use warp::Filter;

use quinn::crypto::rustls::QuicServerConfig;
use rustls::ServerConfig as RustlsServerConfig;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    init_crypto();
    let root = env!("CARGO_MANIFEST_DIR");
    let certs = generate_roxy_root_ca().unwrap();

    let dir = PathBuf::from(format!("{root}/assets"));

    let root = warp::path::end().map(|| "hello");
    let routes = root.or(warp::fs::dir(dir.clone()));

    let http_addr: SocketAddr = ([127, 0, 0, 1], 8000).into();
    let http_server = warp::serve(routes.clone()).bind(http_addr);

    let https_addr: SocketAddr = ([127, 0, 0, 1], 8001).into();

    let (leaf_cert, _key_pair) = certs
        .sign_leaf_mult(
            "localhost",
            vec!["localhost".to_string(), "127.0.0.1".to_string()],
        )
        .unwrap();

    let https_server = warp::serve(routes)
        .tls()
        .key(certs.key_pair().serialize_pem())
        .cert(leaf_cert.pem())
        .bind(https_addr);

    let ws_server = start_ws_server(8002);
    let wss_server = start_wss_server(8003);

    let dir = dir.clone();
    let http3 = tokio::spawn(async move {
        let mut server_crypto = RustlsServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![leaf_cert.der().clone()],
                PrivateKeyDer::try_from(_key_pair.serialize_der()).unwrap(),
            )
            .unwrap();

        server_crypto.alpn_protocols = vec![b"h3".to_vec()];

        let opt = SocketAddr::from(([127, 0, 0, 1], 8004));
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto).unwrap(),
        ));
        let endpoint = quinn::Endpoint::server(server_config, opt).unwrap();

        while let Some(new_conn) = endpoint.accept().await {
            info!("New connection being attempted");

            let dir = dir.clone();
            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        info!("new connection established");

                        let mut h3_conn =
                            h3::server::Connection::new(h3_quinn::Connection::new(conn))
                                .await
                                .unwrap();

                        loop {
                            let dir = dir.clone();
                            match h3_conn.accept().await {
                                Ok(Some(resolver)) => {
                                    tokio::spawn(async {
                                        if let Err(e) = handle_request(resolver, Some(dir)).await {
                                            error!("handling request failed: {}", e);
                                        }
                                    });
                                }
                                Ok(None) => {
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
    });

    // TODO: http1 http2 http1/2
    println!("HTTP  →   http://localhost:8000");
    println!("HTTPS →   https://localhost:8001");
    println!("HTTP3 →   https://localhost:8004");
    println!("WS    →   ws://localhost:8002");
    println!("WSS   →   wss://localhost:8003");

    let _ = tokio::join!(http_server, https_server, ws_server, wss_server, http3);
    Ok(())
}

async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
    serve_root: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream) = resolver.resolve_request().await?;
    let path = req.uri().path();

    if path == "/" {
        let response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header("content-type", "text/plain")
            .body(())
            .unwrap();

        stream.send_response(response).await?;
        stream.send_data(Bytes::from_static(b"hello")).await?;
        return Ok(());
    }

    let (status, to_serve) = match serve_root.as_deref() {
        None => (http::StatusCode::OK, None),
        Some(_) if req.uri().path().contains("..") => (http::StatusCode::NOT_FOUND, None),
        Some(root) => {
            let to_serve = root.join(req.uri().path().strip_prefix('/').unwrap_or(""));
            match File::open(&to_serve).await {
                Ok(file) => (http::StatusCode::OK, Some(file)),
                Err(e) => {
                    error!("failed to open: \"{}\": {}", to_serve.to_string_lossy(), e);
                    (http::StatusCode::NOT_FOUND, None)
                }
            }
        }
    };

    let resp = http::Response::builder().status(status).body(()).unwrap();

    match stream.send_response(resp).await {
        Ok(_) => {
            info!("successfully respond to connection");
        }
        Err(err) => {
            error!("unable to send response to connection peer: {:?}", err);
        }
    }

    if let Some(mut file) = to_serve {
        loop {
            let mut buf = BytesMut::with_capacity(4096 * 10);
            if file.read_buf(&mut buf).await? == 0 {
                break;
            }
            stream.send_data(buf.freeze()).await?;
        }
    }

    Ok(stream.finish().await?)
}
