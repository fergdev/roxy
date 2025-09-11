use std::{error::Error, io, net::UdpSocket, sync::Arc};

use bytes::{Buf, Bytes, BytesMut};
use h3::{ext::Protocol, server::RequestResolver};
use http::{
    Method,
    header::{CONTENT_TYPE, HOST},
};
use quinn::{
    EndpointConfig,
    crypto::rustls::{NoInitialCipherSuite, QuicServerConfig},
    default_runtime,
};
use roxy_shared::{
    alpn::{AlpnProtocol, alp_h3},
    client::ClientContext,
    content::{ContentType, encode_body_opt},
    http::HttpError,
    uri::RUri,
};
use rustls::ServerConfig;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::{
    flow::{FlowEvent, InterceptedRequest, InterceptedResponse},
    proxy::{FlowContext, ProxyContext},
};

// TODO: handle this from https://www.ietf.org/archive/id/draft-schinazi-masque-connect-udp-00.html
// If there are multiple proxies involved, proxies along the chain MUST check whether their upstream connection supports HTTP/3 datagrams. If it does not, that proxy MUST remove the "Datagram-Flow-Id" header before forwarding the CONNECT-UDP request.
//

pub enum H3Error {
    RustLs,
    NoCipherSuite,
    Io,
}

impl From<rustls::Error> for H3Error {
    fn from(_value: rustls::Error) -> Self {
        H3Error::RustLs
    }
}
impl From<NoInitialCipherSuite> for H3Error {
    fn from(_value: NoInitialCipherSuite) -> Self {
        H3Error::NoCipherSuite
    }
}
impl From<std::io::Error> for H3Error {
    fn from(_value: std::io::Error) -> Self {
        H3Error::Io
    }
}

pub async fn start_h3(cxt: ProxyContext, udp_socket: UdpSocket) -> Result<JoinHandle<()>, H3Error> {
    let addr = udp_socket.local_addr()?;
    let (leaf, kp) = cxt.ca.local_leaf();
    let mut tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![leaf], kp)?;

    tls_config.alpn_protocols = alp_h3();

    let runtime = default_runtime().ok_or_else(|| io::Error::other("no async runtime found"))?;

    let udp_socket = runtime.wrap_udp_socket(udp_socket)?;

    let qsc = QuicServerConfig::try_from(tls_config)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(qsc));
    let endpoint = quinn::Endpoint::new_with_abstract_socket(
        EndpointConfig::default(),
        Some(server_config),
        udp_socket,
        runtime,
    )?;
    let handle = tokio::spawn(async move {
        debug!("Accepting H3 on {}", addr);

        while let Some(new_conn) = endpoint.accept().await {
            let cxt = cxt.clone();
            tokio::spawn(async {
                if let Err(e) = do_conn(new_conn, cxt).await {
                    error!("H3 conn err {e}");
                }
            });
        }
        error!("HTTP/3 server stopped accepting connections");
    });

    Ok(handle)
}

async fn do_conn(new_conn: quinn::Incoming, cxt: ProxyContext) -> Result<(), Box<dyn Error>> {
    match new_conn.await {
        Ok(conn) => {
            let addr = conn.remote_address();
            trace!("H3 conn {addr}");
            let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;

            let resolver = match h3_conn.accept().await? {
                Some(res) => res,
                None => return Err(Box::new(std::io::Error::other("Resolver was none"))),
            };

            let target_uri = handle_connect(resolver).await?;
            let flow_cxt = FlowContext::new(addr, target_uri, cxt);

            loop {
                match h3_conn.accept().await {
                    Ok(Some(resolver)) => {
                        let Ok((req, mut stream)) = resolver.resolve_request().await else {
                            warn!("Failed to resolve_request");
                            continue;
                        };

                        let mut bytes = BytesMut::new();
                        while let Ok(Some(chunk)) = stream.recv_data().await {
                            bytes.extend(chunk.chunk());
                        }

                        stream.recv_trailers().await?;

                        let mut intercepted_request = InterceptedRequest::from_http(
                            req.uri().into(),
                            AlpnProtocol::Http3,
                            req.into_parts().0,
                            bytes.freeze(),
                            None,
                        );

                        let response = flow_cxt
                            .proxy_cxt
                            .script_engine
                            .intercept_request(&mut intercepted_request)
                            .await?;

                        let req = intercepted_request.request()?;
                        let flow_id = flow_cxt
                            .proxy_cxt
                            .flow_store
                            .new_flow_cxt(&flow_cxt, intercepted_request.clone())
                            .await;

                        if let Some(response) = response {
                            flow_cxt
                                .proxy_cxt
                                .flow_store
                                .post_event(flow_id, FlowEvent::Response(response.clone()));

                            let resp = response.response_builder();
                            stream.send_response(resp.body(())?).await?;
                            stream.send_data(response.body).await?;
                            if let Some(trailers) = response.trailers {
                                stream.send_trailers(trailers).await?;
                            }
                            stream.finish().await?;
                            continue;
                        }

                        let client = ClientContext::builder()
                            .with_roxy_ca(flow_cxt.proxy_cxt.ca.clone())
                            .build();
                        let resp = client.request(req).await?;

                        let mut intercepted_response =
                            InterceptedResponse::from_http(resp.parts, resp.body, resp.trailers);

                        flow_cxt
                            .proxy_cxt
                            .script_engine
                            .intercept_response(&intercepted_request, &mut intercepted_response)
                            .await?;

                        let resp = intercepted_response.response_builder();
                        let body = encode_body_opt(
                            intercepted_response.body.clone(),
                            &intercepted_response.encoding,
                        )?;
                        let trailers = intercepted_response.trailers.clone();

                        flow_cxt
                            .proxy_cxt
                            .flow_store
                            .post_event(flow_id, FlowEvent::Response(intercepted_response.clone()));

                        stream.send_response(resp.body(())?).await?;
                        stream.send_data(body).await?;
                        if let Some(trailers) = trailers {
                            stream.send_trailers(trailers).await?;
                        }
                        stream.finish().await?;
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
    Ok(())
}

async fn handle_connect<C>(resolver: RequestResolver<C, Bytes>) -> Result<RUri, Box<dyn Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (req, mut stream) = resolver.resolve_request().await?;
    debug!(?req, "Received request");
    let req_host = req.headers().get(HOST);

    let target_uri = match req_host {
        Some(host) => host.to_str()?.parse()?,
        None => return Err(Box::new(HttpError::BadHost)),
    };

    match req.method() {
        &Method::CONNECT if req.extensions().get::<Protocol>() == Some(&Protocol::CONNECT_UDP) => {
            let response = http::Response::builder()
                .status(http::StatusCode::OK)
                .header(CONTENT_TYPE, ContentType::Text.to_default_str())
                .body(())?;
            stream.send_response(response).await?;
            stream.finish().await?;

            Ok(target_uri)
        }
        _ => {
            let response = http::Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .header(CONTENT_TYPE, ContentType::Text.to_default_str())
                .body(())?;
            stream.send_response(response).await?;
            stream.finish().await?;
            Err(Box::new(HttpError::ProxyConnect))
        }
    }
}
