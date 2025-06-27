use clap::Parser;
use interceptor::ScriptEngine;

pub mod app;
pub mod certs;
pub mod event;
pub mod flow;
pub mod interceptor;
pub mod logging;
pub mod proxy;

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

    let mut app = app::App::new();
    let tx = app.events.sender();

    // TODO: assert is filee and is lua
    let script_engine = args.script.map(|s| ScriptEngine::new(s).unwrap());

    let _ = proxy::start_proxy(args.port, tx, roxy_certs, script_engine);
    color_eyre::install().unwrap();
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal).await;
    ratatui::restore();
    result
}
