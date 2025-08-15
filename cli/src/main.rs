#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::derivable_impls)]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use roxy_cli::{
    app,
    config::ConfigManager,
    logging, notify_debug, notify_error, notify_info, notify_trace, notify_warn,
    ui::{framework::notify::Notifier, log::UiLogLayer},
};
use roxy_proxy::{
    flow::FlowStore,
    interceptor::{self, ScriptEngine},
    proxy::ProxyManager,
};
use roxy_shared::tls::TlsConfig;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let log_layer = UiLogLayer::new(log_buffer.clone());

    let notifier = Notifier::new();

    if let Err(e) = logging::initialize_logging_with_layer(Some(log_layer)) {
        eprintln!("Err {e}");
        return Ok(());
    }
    let config_manager = match ConfigManager::new() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Err {err}");
            return Ok(());
        }
    };

    let roxy_certs = match roxy_shared::generate_roxy_root_ca() {
        Ok(certs) => certs,
        Err(err) => {
            eprintln!("{err}");
            return Ok(());
        }
    };

    let flow_store = FlowStore::new();
    let cfg = config_manager.rx.borrow();

    let (notify_tx, mut notify_rx) = mpsc::channel::<interceptor::FlowNotify>(16);

    let notify_handle = tokio::spawn(async move {
        while let Some(notifcation) = notify_rx.recv().await {
            match notifcation.level {
                0 => notify_info!("{}", notifcation.msg),
                1 => notify_warn!("{}", notifcation.msg),
                2 => notify_debug!("{}", notifcation.msg),
                3 => notify_trace!("{}", notifcation.msg),
                _ => notify_error!("{}", notifcation.msg),
            }
        }
    });
    let mut script_engine = match ScriptEngine::new_notify(notify_tx).await {
        Ok(se) => se,
        Err(err) => {
            eprintln!("SE error {err}");
            return Ok(());
        }
    };

    if let Some(script) = cfg.app.proxy.script_path.clone() {
        info!("Setting script!");
        if let Err(e) = script_engine.load_script_path(script).await {
            notify_error!("Failed to load script {e}");
        }
    }

    let tls_config = TlsConfig::default();
    let mut proxy_manager = ProxyManager::new(
        cfg.app.proxy.port,
        roxy_certs,
        script_engine,
        tls_config,
        flow_store.clone(),
    );

    if let Err(err) = proxy_manager.start_all().await {
        eprintln!("{err}");
        return Ok(());
    }

    drop(cfg);

    let mut app = app::App::new(
        proxy_manager,
        config_manager,
        flow_store.clone(),
        log_buffer,
        notifier,
    );
    if let Err(err) = app.run().await {
        eprintln!("{err:?}");
    }
    notify_handle.abort();
    ratatui::restore();
    Ok(())
}
