pub mod args;
pub mod color;
pub mod keys;
pub mod manager;

use config::ConfigError;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use tracing::{debug, error};

use color_eyre::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::config::color::RoxyColors;
use crate::config::keys::KeyBindings;

const CONFIG: &str = include_str!("../../../.config/config.json");

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl RoxyConfig {
    fn new() -> Result<Self, ConfigError> {
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

        let mut cfg: Self = builder.build()?.try_deserialize().map_err(|e| {
            error!("Failed to deserialize config: {}", e);
            ConfigError::Message(format!("Failed to deserialize config: {e}"))
        })?;

        cfg.app.data_dir = data_dir;
        cfg.app.config_dir = config_dir;

        Ok(cfg)
    }
}

pub(crate) fn get_config_dir() -> PathBuf {
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("roxy");
    }

    ProjectDirs::from("com", "roxy", "Roxy")
        .map(|d| d.config_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".config"))
}

pub(crate) fn get_data_dir() -> PathBuf {
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
