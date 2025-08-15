#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::error::Error;

use bytes::Bytes;
use clap::{Parser, command};
use http::{
    Version,
    header::{ACCEPT, HOST, USER_AGENT},
};
use http_body_util::{Empty, Full, combinators::BoxBody};
use once_cell::sync::OnceCell;
use roxy_shared::{client::ClientContext, crypto::init_crypto, generate_roxy_root_ca, uri::RUri};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

pub static INIT_LOGGER: OnceCell<()> = OnceCell::new();

pub fn init_logging() {
    INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::fmt()
            .without_time()
            .with_line_number(true)
            .with_env_filter(EnvFilter::from_default_env())
            .with_test_writer()
            .init();
    });
}
static RURL_USER_AGENT: &str = "rurl/0.1";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    uri: RUri,

    #[arg(short, long)]
    proxy: Option<RUri>,

    #[arg(short, long, default_value = "*/*")]
    accept: String,

    #[arg(short, long, default_value = "GET")]
    request: String,

    #[arg(short, long, default_value = None)]
    data: Option<String>,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    init_logging();
    init_crypto();

    let args = Args::try_parse();

    let args = match args {
        Ok(args) => args,
        Err(e) => {
            error!("Error parsing args {e}");
            return Err(Box::<dyn Error>::from(e));
        }
    };
    debug!("{args:?}");
    let ca = generate_roxy_root_ca()?;
    let builder = http::Request::builder()
        .method(args.request.as_str())
        .version(Version::HTTP_11)
        .uri(args.uri.clone())
        .header(HOST, args.uri.host_port())
        .header(ACCEPT, args.accept)
        .header(USER_AGENT, RURL_USER_AGENT);

    let req = if let Some(body) = args.data {
        builder.body(BoxBody::new(Full::new(Bytes::from(body))))
    } else {
        builder.body(BoxBody::new(Empty::<Bytes>::new()))
    }?;

    let mut builder = ClientContext::builder().with_roxy_ca(ca.clone());
    if let Some(proxy) = args.proxy {
        builder = builder.with_proxy(proxy);
    }

    let client = builder.build();
    let resp = client.request(req).await?;

    // TODO: Format output using parsers.
    let status = resp.parts.status;
    let version = resp.parts.version;
    info!("{status}, {version:?}");

    for (k, v) in resp.parts.headers.iter() {
        info!("H {k}: {v:?}");
    }

    let body = resp.body;
    info!("{body:?}");

    if let Some(trailers) = resp.trailers {
        info!("Trailers ....");
        for (k, v) in trailers {
            info!("{k:?}: {v:?}");
        }
    }

    Ok(())
}
