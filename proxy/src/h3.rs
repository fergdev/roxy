use std::{net::SocketAddr, sync::Arc};

use bytes::{Buf, Bytes, BytesMut};
use futures::future::{self};
use h3::server::RequestResolver;
use http::{Method, Uri, header::HOST};
use quinn::crypto::rustls::QuicServerConfig;
use roxy_shared::{RoxyCA, init_crypto};
use rustls::{ServerConfig, pki_types::PrivateKeyDer};
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::{
    flow::{
        FlowConnection, FlowKind, FlowStore, Http2Flow, InterceptedRequest, InterceptedResponse,
        Scheme,
    },
    interceptor::ScriptEngine,
};

static ALPN: &[u8] = b"h3";

pub fn start_h3(
    port: u16,
    roxy_ca: Arc<RoxyCA>,
    script_engine: Option<ScriptEngine>,
    flow_store: FlowStore,
) -> std::io::Result<()> {
    info!("Init h3");
    tokio::spawn(async move {
        init_crypto();
        info!("tls_config");

        let (leaf, kp) = roxy_ca
            .sign_leaf_mult(
                "localhost",
                vec!["localhost".to_string(), "127.0.0.1".to_string()],
            )
            .unwrap();

        let mut tls_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![leaf.der().clone()],
                PrivateKeyDer::try_from(kp.serialize_der()).unwrap(),
            )
            .unwrap();

        tls_config.alpn_protocols = vec![b"h3".to_vec()];

        let opt = SocketAddr::from(([127, 0, 0, 1], port));
        info!("server_config");
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(tls_config).unwrap(),
        ));

        info!("endpoint");
        let endpoint = quinn::Endpoint::server(server_config, opt).unwrap();

        info!("Accepting h3 on {}", opt);
        while let Some(new_conn) = endpoint.accept().await {
            let roxy_ca = roxy_ca.clone();
            let flow_store = flow_store.clone();
            let script_engine = script_engine.clone();
            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        let addr = conn.remote_address();
                        info!("new connection established");
                        let mut h3_conn =
                            h3::server::Connection::new(h3_quinn::Connection::new(conn))
                                .await
                                .unwrap();

                        let requested_addr = match h3_conn.accept().await {
                            Ok(Some(resolver)) => {
                                info!("Handling connect");
                                match handle_conn(resolver).await {
                                    Ok(addr) => addr,
                                    Err(err) => {
                                        error!("handling request failed: {}", err);
                                        return;
                                    }
                                }
                            }
                            _ => {
                                return;
                            }
                        };

                        let (tx, rx) = tokio::sync::mpsc::channel(32);
                        let handle =
                            match connect_upstream(requested_addr.clone(), roxy_ca.clone(), rx)
                                .await
                            {
                                Ok(handle) => handle,
                                Err(e) => {
                                    error!("Error connecting upstream {e}");
                                    return;
                                }
                            };
                        loop {
                            match h3_conn.accept().await {
                                Ok(Some(resolver)) => {
                                    let (req, mut stream) =
                                        resolver.resolve_request().await.unwrap();

                                    let mut bytes = BytesMut::new();
                                    while let Some(chunk) = stream.recv_data().await.unwrap() {
                                        bytes.extend(chunk.chunk());
                                    }
                                    let bytes = bytes.freeze();
                                    let mut ir = InterceptedRequest::from_http(
                                        Scheme::Https,
                                        &req.into_parts().0,
                                        requested_addr.host().unwrap_or("localhost"),
                                        requested_addr.port_u16().unwrap_or(443),
                                        &bytes,
                                    );

                                    script_engine
                                        .as_ref()
                                        .map(|engine| engine.intercept_request(&mut ir));

                                    let clone = ir.clone(); // HACK: don't clone

                                    let conn = FlowConnection { addr };
                                    let flow = flow_store.new_flow(conn).await;
                                    let mut guard = flow.write().await;
                                    guard.kind = FlowKind::Http2(Http2Flow::new(ir));
                                    drop(guard);
                                    flow_store.notify();

                                    let (back_tx, back_rx) = tokio::sync::oneshot::channel();
                                    tx.send((clone, back_tx)).await.unwrap();

                                    let mut resp = match back_rx.await {
                                        Ok(resp) => resp,
                                        Err(err) => {
                                            error!("Error back_tx {}", err);
                                            return;
                                        }
                                    };

                                    script_engine
                                        .as_ref()
                                        .map(|engine| engine.intercept_response(&mut resp));

                                    let clone = resp.clone(); // HACK: don't clone

                                    let mut guard = flow.write().await;
                                    match &mut guard.kind {
                                        FlowKind::Http2(http2_flow) => {
                                            http2_flow.response = Some(resp);
                                        }
                                        _ => panic!("Wrong kind"),
                                    }
                                    drop(guard);
                                    flow_store.notify();

                                    let response = http::Response::builder()
                                        .status(http::StatusCode::OK)
                                        .header("content-type", "text/plain")
                                        .body(())
                                        .unwrap();

                                    stream.send_response(response).await.unwrap();
                                    stream.send_data(clone.body).await.unwrap();

                                    stream.finish().await.unwrap();
                                }

                                Ok(None) => {
                                    break;
                                }

                                Err(err) => {
                                    error!("error on accept {}", err);
                                }
                            }
                        }
                        handle.abort();
                    }
                    Err(err) => {
                        error!("accepting connection failed: {:?}", err);
                    }
                }
            });
        }
        error!("HTTP/3 server stopped accepting connections");
    });
    Ok(())
}

async fn handle_conn<C>(resolver: RequestResolver<C, Bytes>) -> anyhow::Result<Uri>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream) = resolver.resolve_request().await?;

    let host: Option<Uri> = match req.headers().get(HOST).map(|h| h.to_str()) {
        Some(Ok(s)) => s.parse::<Uri>().map(Some).unwrap_or(None),
        _ => None,
    };

    if req.method() == Method::CONNECT && host.is_some() {
        let response = http::Response::builder()
            .status(http::StatusCode::OK)
            .header("content-type", "text/plain")
            .body(())
            .unwrap();
        stream.send_response(response).await?;
        stream.finish().await?;

        Ok(host.unwrap()) // TODO: not unwrap
    } else {
        let response = http::Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header("content-type", "text/plain")
            .body(())
            .unwrap();
        stream.send_response(response).await?;
        stream
            .send_data(Bytes::from_static(b"Proxy needs connect"))
            .await?;
        stream.finish().await?;
        Err(anyhow::anyhow!("Not a connect method"))
    }
}

async fn connect_upstream(
    requested_addr: Uri,
    roxy_ca: Arc<RoxyCA>,
    mut tx: tokio::sync::mpsc::Receiver<(
        InterceptedRequest,
        tokio::sync::oneshot::Sender<InterceptedResponse>,
    )>,
) -> anyhow::Result<JoinHandle<()>> {
    info!("H3 Proxy req_addr {requested_addr}");
    let mut roots = rustls::RootCertStore::empty();

    let cert_result = rustls_native_certs::load_native_certs();

    for err in cert_result.errors.iter() {
        error!("Load cert error {err}");
    }
    if !cert_result.errors.is_empty() {
        panic!("Cert load errors");
    }

    for cert in cert_result.certs {
        if let Err(e) = roots.add(cert) {
            error!("failed to parse trust anchor: {}", e);
        }
    }

    roots.add(roxy_ca.ca_der.clone())?;

    info!("Tls config");
    let mut tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    tls_config.enable_early_data = true;
    tls_config.alpn_protocols = vec![ALPN.into()];

    let client_addr: SocketAddr = "[::]:0".parse()?;
    let mut client_endpoint = h3_quinn::quinn::Endpoint::client(client_addr)?;

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));
    client_endpoint.set_default_client_config(client_config);

    let handle = tokio::spawn(async move {
        let host = requested_addr.host().unwrap_or("localhost");
        let port = requested_addr.port_u16().unwrap_or(443);
        let formatted_host = format!("{host}:{port}");
        let addr = tokio::net::lookup_host((host, port))
            .await
            .unwrap()
            .next()
            .ok_or(anyhow::anyhow!("dns found no addresses"))
            .unwrap();

        info!("quinn connect {addr} {}", host);
        let conn = client_endpoint.connect(addr, host).unwrap().await.unwrap();

        info!("QUIC connection established");
        let quinn_conn = h3_quinn::Connection::new(conn);

        info!("QUIC connection established");
        let (mut driver, mut send_request) = h3::client::new(quinn_conn).await.unwrap();

        info!("QuicClientConfig created");
        let _drive = tokio::spawn(async move {
            let a = future::poll_fn(|cx| driver.poll_close(cx)).await;
            error!("Poll close {}", a);
        });

        info!("awaiting request");
        while let Some((_req, reply)) = tx.recv().await {
            info!("going up stream");
            let req = http::Request::builder()
                .method(Method::GET)
                .header("Host", formatted_host.clone().as_str())
                .uri(formatted_host.clone())
                .body(())
                .unwrap();

            info!("Sending request");
            let mut stream = send_request.send_request(req).await.unwrap();

            info!("awaiting res");
            stream.finish().await.unwrap();
            let resp = stream.recv_response().await.unwrap();
            let parts = resp.into_parts();
            let mut bytes = BytesMut::new();
            while let Some(chunk) = stream.recv_data().await.unwrap() {
                bytes.extend(chunk.chunk());
            }
            let bytes = bytes.freeze();
            let chunk = String::from_utf8_lossy(&bytes);
            info!("chunky {}", chunk);

            let resp = InterceptedResponse::from_http(&parts.0, &bytes);

            reply.send(resp).unwrap();
        }
    });

    Ok(handle)
}
