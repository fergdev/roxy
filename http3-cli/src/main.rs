use std::path::PathBuf;

use http::Method;
use http3_cli::h3_with_proxy;
use rustls::pki_types::{CertificateDer, pem::PemObject};
use structopt::StructOpt;
use tracing::info;

// TODO: handle this from https://www.ietf.org/archive/id/draft-schinazi-masque-connect-udp-00.html
// If there are multiple proxies involved, proxies along the chain MUST check whether their upstream connection supports HTTP/3 datagrams. If it does not, that proxy MUST remove the "Datagram-Flow-Id" header before forwarding the CONNECT-UDP request.
//
#[derive(StructOpt, Debug)]
#[structopt(name = "server")]
struct Opt {
    #[structopt(
        long,
        short,
        default_value = "examples/ca.cert",
        help = "Certificate of CA who issues the server certificate"
    )]
    pub ca: PathBuf,

    // #[structopt(name = "keylogfile", long)]
    // pub key_log_file: bool,
    #[structopt()]
    pub proxy: String,

    #[structopt()]
    pub target: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let opt = Opt::from_args();
    let proxy_uri = opt.proxy.parse::<http::Uri>()?;
    if proxy_uri.scheme() != Some(&http::uri::Scheme::HTTPS) {
        Err("uri scheme must be 'https'")?;
    }

    let taget_uri = opt.target.parse::<http::Uri>()?;

    let req = http::Request::builder()
        .method(Method::CONNECT)
        .header("Host", taget_uri.authority().unwrap().to_string())
        .body(())
        .unwrap();

    let ca_der = CertificateDer::from_pem_file(opt.ca).unwrap();

    let resp = h3_with_proxy(proxy_uri, taget_uri, ca_der, req).await?;

    info!("Response {} ", resp.status());

    Ok(())
}
