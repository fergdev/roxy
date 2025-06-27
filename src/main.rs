use clap::Parser;
use tracing::debug;

pub mod app;
pub mod app2;
pub mod event;
pub mod keymap;
pub mod logging;
pub mod proxy;
pub mod ui;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    logging::initialize_logging().unwrap();
    let args = Args::parse();

    let mut app = app2::App::new();
    let tx = app.events.sender();

    let host = format!("127.0.0.1:{}", args.port);
    let _ = proxy::start_proxy(host.as_str(), tx);
    color_eyre::install().unwrap();
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal).await;
    debug!("wellllllllll");
    ratatui::restore();
    debug!("oh wellllllllll");
    result
}
