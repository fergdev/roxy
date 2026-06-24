pub mod color;
pub mod keys;

use clap::Parser;
use config::ConfigError;
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use tokio::sync::watch;
use tracing::{debug, error};

use color_eyre::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::config::color::RoxyColors;
use crate::config::keys::KeyBindings;
use crate::notify_error;

const CONFIG: &str = include_str!("../../../.config/config.json");

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about=None)]
pub struct RoxyArgs {
    #[arg(short, long)]
    port: Option<u16>,

    #[arg(short, long)]
    script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub confirm_quit: bool,
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
    #[serde(default)]
    pub proxy: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    pub port: u16,
    pub ca_cert_path: Option<PathBuf>,
    pub script_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoxyConfig {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
    pub colors: RoxyColors,
    pub typography: Typography,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Typography {
    pub font_size: u16,
}

#[derive(Clone, Debug)]
pub struct ConfigManager {
    pub tx: watch::Sender<RoxyConfig>,
    pub rx: watch::Receiver<RoxyConfig>,
}

#[derive(Debug)]
pub enum RoxyConfigError {
    ReadError,
    WriteError,
    ConfigError,
    Deserialize,
    InvalidFormat,
}

impl From<ConfigError> for RoxyConfigError {
    fn from(_value: ConfigError) -> Self {
        RoxyConfigError::ConfigError
    }
}

impl Error for RoxyConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl Display for RoxyConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ConfigManager {
    pub fn new() -> Result<Self, RoxyConfigError> {
        let args = RoxyArgs::parse();
        let mut config = Self::read_from_disk()?;

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

    fn read_from_disk() -> Result<RoxyConfig, ConfigError> {
        let rc = RoxyConfig::new()?;
        Ok(rc)
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
        debug!("Persisting updated config: {:?}", updated);
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

impl RoxyConfig {
    fn new() -> Result<Self, config::ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        debug!("Using data directory: {:?}", data_dir.as_path());
        debug!("Using config directory: {:?}", config_dir.as_path());
        let mut builder = config::Config::builder()
            .add_source(config::File::from_str(CONFIG, config::FileFormat::Json5))
            .add_source(
                config::Environment::with_prefix("ROXY")
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(" "),
            )
            .set_default("data_dir", data_dir.to_str())?
            .set_default("config_dir", config_dir.to_str())?;

        let config_files = [
            ("config.toml", config::FileFormat::Toml),
            ("config.json", config::FileFormat::Json),
        ];
        let mut found_config = false;
        for (file, format) in &config_files {
            let source = config::File::from(config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
            if config_dir.join(file).exists() {
                found_config = true
            }
        }

        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        let cfg: Self = builder.build()?.try_deserialize().map_err(|e| {
            error!("Failed to deserialize config: {}", e);
            config::ConfigError::Message(format!("Failed to deserialize config: {e}"))
        })?;

        Ok(cfg)
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("roxy");
    }

    ProjectDirs::from("com", "roxy", "Roxy")
        .map(|d| d.config_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".config"))
}

pub fn get_data_dir() -> PathBuf {
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("roxy");
    }

    ProjectDirs::from("com", "roxy", "Roxy")
        .map(|d| d.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".data"))
}
