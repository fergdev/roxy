use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use clap::Parser;
use roxy::{
    app, certs, config::Config, flow::FlowStore, interceptor::ScriptEngine, logging, proxy,
    ui::log::UiLogLayer,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value_t = 6969)]
    port: u16,

    #[arg(short, long)]
    script: Option<String>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let log_layer = UiLogLayer::new(log_buffer.clone());

    let config = Config::new().unwrap();
    logging::initialize_logging_with_layer(Some(log_layer)).unwrap();

    let args = Args::parse();
    let roxy_certs = certs::generate_roxy_root_ca().unwrap();

    let flow_store = FlowStore::new();

    let script_engine = args.script.map(|s| ScriptEngine::new(s).unwrap());
    let _ = proxy::start_proxy(args.port, roxy_certs, script_engine, flow_store.clone());

    color_eyre::install().unwrap();
    // let mut terminal = ratatui::init();

    let mut app = app::App::new(config, flow_store.clone(), log_buffer);
    let result = app.run().await;
    ratatui::restore();
    result
}
