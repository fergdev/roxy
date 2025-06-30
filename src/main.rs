use clap::Parser;
use roxy::{app, certs, flow::FlowStore, interceptor::ScriptEngine, logging, proxy};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    port: u16,

    #[arg(short, long)]
    script: Option<String>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    logging::initialize_logging().unwrap();

    let args = Args::parse();
    let roxy_certs = certs::generate_roxy_root_ca().unwrap();

    let flow_store = FlowStore::new();

    let script_engine = args.script.map(|s| ScriptEngine::new(s).unwrap());
    let _ = proxy::start_proxy(args.port, roxy_certs, script_engine, flow_store.clone());

    color_eyre::install().unwrap();
    let mut terminal = ratatui::init();

    let app = app::App::new(flow_store.clone());
    let result = app.run(&mut terminal).await;
    ratatui::restore();
    result
}
