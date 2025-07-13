#![allow(clippy::derivable_impls)]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use roxy_cli::{
    app,
    config::ConfigManager,
    logging, notify_error, notify_info,
    ui::{framework::notify::Notifier, log::UiLogLayer},
};
use roxy_proxy::{flow::FlowStore, interceptor::ScriptEngine, proxy::start_proxy};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let log_layer = UiLogLayer::new(log_buffer.clone());

    let notifier = Notifier::new();

    logging::initialize_logging_with_layer(Some(log_layer)).unwrap();

    let config = ConfigManager::new().unwrap();

    let roxy_certs = roxy_shared::generate_roxy_root_ca().unwrap();

    let flow_store = FlowStore::new();
    let cfg = config.rx.borrow();

    let script_engine = cfg
        .app
        .proxy
        .script_path
        .clone()
        .map(|s| ScriptEngine::new(s).unwrap());

    if script_engine.is_some() {
        notify_info!("Some engine");
    } else {
        notify_error!("No engine");
    }

    let _ = start_proxy(
        cfg.app.proxy.port,
        roxy_certs,
        script_engine,
        flow_store.clone(),
    );
    drop(cfg);

    color_eyre::install().unwrap();

    let mut app = app::App::new(config, flow_store.clone(), log_buffer, notifier);
    let result = app.run().await;
    ratatui::restore();
    result
}
