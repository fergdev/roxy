use std::convert::Infallible;

use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;
use http::uri::Scheme;
use http_body_util::BodyExt;
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use roxy_shared::alpn::AlpnProtocol;
use roxy_shared::client::ClientContext;
use roxy_shared::content::ContentType;
use roxy_shared::http::HttpError;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::debug;
use tracing::trace;

type H1ServerBuilder = hyper::server::conn::http1::Builder;
type H2ServerBuilder<TokioIo> = hyper::server::conn::http2::Builder<TokioIo>;

use crate::flow::FlowEvent;
use crate::flow::FlowEventEmitter;
use crate::flow::InterceptedRequest;
use crate::flow::InterceptedResponse;
use crate::proxy::FlowContext;

pub async fn handle_http(
    flow_cxt: FlowContext,
    client_request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, HttpError> {
    proxy(flow_cxt, AlpnProtocol::None, Scheme::HTTP, client_request).await
}

pub async fn handle_https<S>(
    flow_cxt: FlowContext,
    client_stream: S,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    trace!("Spawning HS client connection handler");
    H1ServerBuilder::new()
        .title_case_headers(true)
        .keep_alive(true)
        .serve_connection(
            TokioIo::new(client_stream),
            service_fn(|req| proxy(flow_cxt.clone(), AlpnProtocol::Http1, Scheme::HTTPS, req)),
        )
        .await?;
    Ok(())
}

pub async fn handle_h2<S>(
    flow_cxt: FlowContext,
    client_stream: S,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    trace!("Spawning H2 client connection handler");
    H2ServerBuilder::new(TokioExecutor::new())
        .serve_connection(
            TokioIo::new(client_stream),
            service_fn(|req| proxy(flow_cxt.clone(), AlpnProtocol::Http2, Scheme::HTTPS, req)),
        )
        .await?;
    Ok(())
}

async fn proxy(
    flow_cxt: FlowContext,
    alpn: AlpnProtocol,
    scheme: Scheme,
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, HttpError> {
    debug!("Proxy {:?}", flow_cxt.target_uri);
    let (parts, body) = req.into_parts();
    let body = body.collect().await?;
    let trailers = body.trailers().cloned();
    let body_bytes = body.to_bytes();

    let uri = match flow_cxt.target_uri.and(&parts.uri, scheme) {
        Ok(uri) => uri,
        Err(_) => return down_stream_error(HttpError::BadHost),
    };

    let mut intercepted = InterceptedRequest::from_http(uri, alpn, parts, body_bytes, trailers);

    let response = match flow_cxt
        .proxy_cxt
        .script_engine
        .intercept_request(&mut intercepted)
        .await
    {
        Ok(req) => req,
        Err(err) => return internal_error(format!("Intercept request error: {err}")),
    };

    let down_stream_req = intercepted.request()?;
    let flow_id = flow_cxt
        .proxy_cxt
        .flow_store
        .new_flow_cxt(&flow_cxt, intercepted)
        .await;

    if let Some(response) = response {
        let resp = response.response()?;
        flow_cxt
            .proxy_cxt
            .flow_store
            .post_event(flow_id, FlowEvent::Response(response));
        return Ok(resp);
    }

    let emitter = FlowEventEmitter::new(flow_id, flow_cxt.proxy_cxt.flow_store.clone());

    let client = ClientContext::builder()
        .with_roxy_ca(flow_cxt.proxy_cxt.ca.clone())
        .with_tls_config(flow_cxt.proxy_cxt.tls_config.clone())
        .with_emitter(Box::new(emitter))
        .build();

    let res = match client.request(down_stream_req).await {
        Ok(res) => res,
        Err(e) => return down_stream_error(e),
    };

    let mut intercepted = InterceptedResponse::from_http(res.parts, res.body, res.trailers);

    if let Err(err) = flow_cxt
        .proxy_cxt
        .script_engine
        .intercept_response(&mut intercepted)
        .await
    {
        return internal_error(format!("Intercept response error: {err}"));
    }

    let resp = intercepted.response()?;
    flow_cxt
        .proxy_cxt
        .flow_store
        .post_event(flow_id, FlowEvent::Response(intercepted));
    Ok(resp)
}

fn internal_error(msg: String) -> Result<Response<BoxBody<Bytes, Infallible>>, HttpError> {
    let body = BoxBody::new(Full::new(Bytes::from(msg)));
    let resp = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, ContentType::Text.to_default_str())
        .body(body)?;
    Ok(resp)
}

fn down_stream_error(error: HttpError) -> Result<Response<BoxBody<Bytes, Infallible>>, HttpError> {
    let body_text = match error {
        HttpError::Io(error) => format!("Io error {error}"),
        HttpError::Alpn => "Invalid ALPN".to_string(),
        HttpError::Hyper(error) => format!("Hyper error {error}"),
        HttpError::HyperUpgrade => "Hyper failed to upgrade down stream connection".to_string(),
        HttpError::Http(error) => format!("HTTP error {error}"),
        HttpError::Uri => "Invalid uri".to_string(),
        HttpError::InvalidDnsName => "Invalid DNS name".to_string(),
        HttpError::Timeout => "Down stream timeout".to_string(),
        HttpError::ProxyConnect => "Proxy Connection failed".to_string(),
        HttpError::TlsError(error) => format!("TLS failed {error}"),
        HttpError::BadHost => "Bad host".to_string(),
    };

    let body = BoxBody::new(Full::new(Bytes::from(body_text)));
    let resp = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, ContentType::Text.to_default_str())
        .body(body)?;
    Ok(resp)
}
