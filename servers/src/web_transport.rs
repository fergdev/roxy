use bytes::Bytes;
use h3::{
    ext::Protocol,
    quic::{self},
    server::Connection,
};

use h3_quinn::quinn::{self, crypto::rustls::QuicServerConfig};
use h3_webtransport::server::{self, WebTransportSession};
use http::Method;
use quinn::{EndpointConfig, default_runtime};
use roxy_shared::{RoxyCA, alpn::alp_h3_all, io::local_udp_socket};
use rustls::ServerConfig as RustlsServerConfig;
use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
    time::Duration,
};
use tokio::pin;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    task::JoinHandle,
};
use tracing::{error, info, trace_span};

pub async fn h3_wt(roxy_ca: &RoxyCA) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let udp_socket = local_udp_socket(None)?;
    h3_wt_socket(udp_socket, roxy_ca).await
}

pub async fn h3_wt_socket(
    udp_socket: UdpSocket,
    roxy_ca: &RoxyCA,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    let addr = udp_socket.local_addr()?;
    let (cert, signing_key) = roxy_ca.local_leaf();
    let mut server_crypto = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], signing_key)?;

    server_crypto.alpn_protocols = alp_h3_all();
    let server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));

    let runtime =
        default_runtime().ok_or_else(|| std::io::Error::other("no async runtime found"))?;
    let endpoint = quinn::Endpoint::new(
        EndpointConfig::default(),
        Some(server_config),
        udp_socket,
        runtime,
    )?;

    info!("listening on {}", addr);

    let handle = tokio::spawn(async move {
        while let Some(new_conn) = endpoint.accept().await {
            trace_span!("New connection being attempted");

            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        info!("new http3 established");
                        let h3_conn = match h3::server::builder()
                            .enable_webtransport(true)
                            .enable_extended_connect(true)
                            .enable_datagram(true)
                            .max_webtransport_sessions(1)
                            .send_grease(true)
                            .build(h3_quinn::Connection::new(conn))
                            .await
                        {
                            Ok(server) => server,
                            Err(e) => {
                                error!("something bad {e}");
                                return;
                            }
                        };
                        if let Err(err) = handle_connection(h3_conn).await {
                            tracing::error!("Failed to handle connection: {err:?}");
                        }
                    }
                    Err(err) => {
                        error!("accepting connection failed: {:?}", err);
                    }
                }
            });

            // shut down gracefully
            // wait for connections to be closed before exiting
            endpoint.wait_idle().await;
        }
    });

    Ok((addr, handle))
}

async fn handle_connection(
    mut conn: Connection<h3_quinn::Connection, Bytes>,
) -> Result<(), Box<dyn Error>> {
    info!("handle_connection");
    loop {
        info!("Accept");
        match conn.accept().await {
            Ok(Some(resolver)) => {
                // TODO: resolve request in a different task to not block the accept loop
                let (req, stream) = match resolver.resolve_request().await {
                    Ok(request) => request,
                    Err(err) => {
                        error!("error resolving request: {err:?}");
                        continue;
                    }
                };
                info!("new request: {:#?}", req);

                let ext = req.extensions();
                match req.method() {
                    &Method::CONNECT if ext.get::<Protocol>() == Some(&Protocol::WEB_TRANSPORT) => {
                        info!("Peer wants to initiate a webtransport session");

                        info!("Handing over connection to WebTransport");

                        let session = WebTransportSession::accept(req, stream, conn)
                            .await
                            .map_err(|e| std::io::Error::other(format!("yeah {e}")))?;
                        info!("Established webtransport session");
                        // 4. Get datagrams, bidirectional streams, and unidirectional streams and wait for client requests here.
                        // h3_conn needs to hand over the datagrams, bidirectional streams, and unidirectional streams to the webtransport session.
                        handle_session_and_echo_all_inbound_messages(session).await?;

                        return Ok(());
                    }
                    _ => {
                        info!(?req, "Received request");
                    }
                }
            }
            // indicating no more streams to be received
            Ok(None) => {
                break;
            }
            Err(err) => {
                error!("Connection errored with {}", err);
                break;
            }
        }
    }
    Ok(())
}

macro_rules! log_result {
    ($expr:expr) => {
        if let Err(err) = $expr {
            tracing::error!("{err:?}");
        }
    };
}

async fn echo_stream<T, R>(send: T, recv: R) -> Result<(), Box<dyn Error>>
where
    T: AsyncWrite,
    R: AsyncRead,
{
    pin!(send);
    pin!(recv);

    info!("Got stream");
    let mut buf = Vec::new();
    recv.read_to_end(&mut buf).await?;

    let message = Bytes::from(buf);

    send_chunked(send, message).await?;

    Ok(())
}

// Used to test that all chunks arrive properly as it is easy to write an impl which only reads and
// writes the first chunk.
async fn send_chunked(
    mut send: impl AsyncWrite + Unpin,
    data: Bytes,
) -> Result<(), Box<dyn Error>> {
    for chunk in data.chunks(4) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("Sending {chunk:?}");
        send.write_all(chunk).await?;
    }

    Ok(())
}

async fn open_bidi_test<S>(mut stream: S) -> Result<(), Box<dyn Error>>
where
    S: Unpin + AsyncRead + AsyncWrite,
{
    info!("Opening bidirectional stream");

    stream
        .write_all(b"Hello from a server initiated bidi stream")
        .await?;

    let mut resp = Vec::new();
    stream.shutdown().await?;
    stream.read_to_end(&mut resp).await?;

    let r = String::from_utf8_lossy(&resp);
    info!("Got response from client: {r}");

    Ok(())
}

/// This method will echo all inbound datagrams, unidirectional and bidirectional streams.
async fn handle_session_and_echo_all_inbound_messages(
    session: WebTransportSession<h3_quinn::Connection, Bytes>,
) -> Result<(), Box<dyn Error>> {
    info!("handle_session_and_echo_all_inbound_messages");
    let session_id = session.session_id();

    // This will open a bidirectional stream and send a message to the client right after connecting!
    let stream = session.open_bi(session_id).await?;

    tokio::spawn(async move { log_result!(open_bidi_test(stream).await) });

    let mut datagram_reader = session.datagram_reader();
    let mut datagram_sender = session.datagram_sender();

    loop {
        info!("asdfasdfasdf");
        tokio::select! {
            datagram = datagram_reader.read_datagram() => {
                let datagram = match datagram {
                    Ok(datagram) => datagram,
                    Err(err) => {
                        tracing::error!("Failed to read datagram: {err:?}");
                        break;
                    }
                };
                tracing::info!("Received datagram: {datagram:?}");
                let datagram = datagram.into_payload();
                datagram_sender.send_datagram(datagram)?;
            }
            uni_stream = session.accept_uni() => {
                let (id, stream) = match uni_stream? {
                    Some(a) => a,
                None => {
                    error!("yeah we out here and not working");
                    return Err(Box::new(std::io::Error::other("bye")));
                }

                };

                let send = session.open_uni(id).await?;
                tokio::spawn( async move { log_result!(echo_stream(send, stream).await); });
            }
            stream = session.accept_bi() => {
                if let Some(server::AcceptedBi::BidiStream(_, stream)) = stream? {
                    let (send, recv) = quic::BidiStream::split(stream);
                    tokio::spawn( async move { log_result!(echo_stream(send, recv).await); });
                }
            }
            else => {
                break
            }
        }
    }

    info!("Finished handling session");

    Ok(())
}
