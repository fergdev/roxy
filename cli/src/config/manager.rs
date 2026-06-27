use clap::Parser;
use std::path::PathBuf;
use tokio::sync::watch;
use tracing::{debug, error, trace};

use color_eyre::Result;

use crate::config::args::RoxyArgs;
use crate::config::{RoxyConfig, RoxyConfigError, get_config_dir};
use crate::notify_error;

#[derive(Clone, Debug)]
pub struct ConfigManager {
    pub tx: watch::Sender<RoxyConfig>,
    pub rx: watch::Receiver<RoxyConfig>,
}

impl ConfigManager {
    pub fn new() -> Result<Self, RoxyConfigError> {
        let args = RoxyArgs::parse();
        let mut config = RoxyConfig::new()?;
        if let Some(port) = args.port {
            config.app.proxy.port = port;
        }
        if let Some(path) = args.script {
            let pg = PathBuf::from(path);
            if pg.is_file() {
                config.app.proxy.script_path = Some(pg);
            } else {
                notify_error!("Invalid script_path: {:?}", pg);
            }
        }

        let (tx, rx) = watch::channel(config);
        let manager = Self { tx, rx };
        manager.spawn_watcher();

        Ok(manager)
    }

    fn spawn_watcher(&self) {
        // TODO: Manage this corrctly
        let _tx = self.tx.clone();
        let _path = get_config_file_path().0;

        // tokio:::::spawn(move || {
        //     let (tx_watcher, rx_watcher) = std::sync::mpsc::channel();
        //     let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx_watcher).unwrap();
        //     watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();
        //
        //     rx_watcher.into_iter().for_each(|res| {
        //         if res.is_ok() {
        //             if let Ok(updated) = Self::read_from_disk() {
        //                 let _ = tx.send(updated);
        //             }
        //         }
        //     });
        // });
    }

    pub fn persist(&self, updated: &RoxyConfig) -> Result<(), RoxyConfigError> {
        trace!("Persisting updated config: {:?}", updated);
        write_config(&updated).map_err(|e| {
            error!("Failed to write config: {}", e);

            RoxyConfigError::WriteError
        })?;

        Ok(())
    }

    pub fn update(&self, new_config: RoxyConfig) -> Result<(), RoxyConfigError> {
        self.tx.send_replace(new_config.clone());
        self.persist(&new_config)?;
        Ok(())
    }
}

fn write_config<T: serde::Serialize>(config: &T) -> Result<(), RoxyConfigError> {
    let (path, format) = get_config_file_path();

    debug!("Writing config to: {:?}", path);
    debug!("Using format: {:?}", format);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| RoxyConfigError::WriteError)?;
    }

    let serialized = match format {
        config::FileFormat::Toml => {
            toml::to_string_pretty(config).map_err(|_| RoxyConfigError::Deserialize)?
        }
        config::FileFormat::Json => {
            serde_json::to_string_pretty(config).map_err(|_| RoxyConfigError::Deserialize)?
        }
        _ => return Err(RoxyConfigError::InvalidFormat),
    };

    std::fs::write(&path, serialized).map_err(|_| RoxyConfigError::WriteError)?;
    Ok(())
}

fn get_config_file_path() -> (PathBuf, config::FileFormat) {
    let config_dir = get_config_dir();

    let config_files = [
        ("config.toml", config::FileFormat::Toml),
        ("config.json", config::FileFormat::Json),
    ];

    let target = config_files
        .iter()
        .map(|(name, format)| (config_dir.join(name), format))
        .find(|(path, _)| path.exists());

    match target {
        Some((p, f)) => (p.clone(), *f),
        None => {
            let fallback_path = config_dir.join("config.json");
            (fallback_path, config::FileFormat::Json)
        }
    }
}
