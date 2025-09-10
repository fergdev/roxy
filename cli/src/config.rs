use clap::Parser;
use config::ConfigError;
use cow_utils::CowUtils;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lazy_static::lazy_static;
use serde::ser::SerializeMap;
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::watch;
use tracing::{debug, error, info};

use color_eyre::Result;
use derive_deref::{Deref, DerefMut};
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::event::{Action, Mode};
use crate::notify_error;

const CONFIG: &str = include_str!("../../.config/config.json");

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").cow_to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

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
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
    #[serde(default)]
    pub proxy: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    pub enabled: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoxyColors {
    #[serde(deserialize_with = "deserialize_color")]
    pub primary: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_primary: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub secondary: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_secondary: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub surface: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_surface: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub background: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_background: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub outline: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub outline_unfocused: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub error: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub success: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub info: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub warn: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub debug: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub trace: Color,
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

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
        info!("Initializing ConfigManager with default config");
        info!("Initializing ConfigManager {}", CONFIG);
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

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map = inner_map
                    .into_iter()
                    .filter_map(|(key_str, cmd)| match parse_key_sequence(&key_str) {
                        Ok(seq) => Some((seq, cmd)),
                        Err(e) => {
                            notify_error!("Failed to parse key '{}': {}", key_str, e);
                            None
                        }
                    })
                    .collect();
                (mode, converted_inner_map)
            })
            .collect();

        Ok(KeyBindings(keybindings))
    }
}
impl Serialize for KeyBindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (mode, bindings) in &self.0 {
            let mode_key = format!("{mode:?}");
            let mut inner = HashMap::new();

            for (key_seq, action) in bindings {
                inner.insert(format_key_sequence(key_seq), action);
            }

            map.serialize_entry(&mode_key, &inner)?;
        }

        map.end()
    }
}

fn format_key_sequence(seq: &[KeyEvent]) -> String {
    seq.iter()
        .map(key_event_to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!("Unable to parse `{raw}`"));
    }
    let raw = if !raw.contains("><") {
        raw.strip_prefix('<').unwrap_or(raw)
    } else {
        raw
    };
    let sequences = raw
        .split("><")
        .map(|seq| {
            if let Some(s) = seq.strip_prefix('<') {
                s
            } else if let Some(s) = seq.strip_suffix('>') {
                s
            } else {
                seq
            }
        })
        .collect::<Vec<_>>();

    sequences.into_iter().map(parse_key_event).collect()
}

pub fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.cow_to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break,
        };
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" => KeyCode::Char('-'),
        "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            if let Some(mut c) = c.chars().next() {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    c = c.to_ascii_uppercase();
                }
                KeyCode::Char(c)
            } else {
                return Err(format!("Unable to parse {raw}"));
            }
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::F(c) => {
            char = format!("f({c})");
            &char
        }
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "esc",
        KeyCode::Null => "",
        KeyCode::CapsLock => "",
        KeyCode::Menu => "",
        KeyCode::ScrollLock => "",
        KeyCode::Media(_) => "",
        KeyCode::NumLock => "",
        KeyCode::PrintScreen => "",
        KeyCode::Pause => "",
        KeyCode::KeypadBegin => "",
        KeyCode::Modifier(_) => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
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

pub fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).unwrap_or("#ffffff".to_string());
    parse_color(&s).map_err(de::Error::custom)
}

pub fn parse_color(s: &str) -> Result<Color, String> {
    let s = s.trim();

    // Named color
    if let Ok(c) = parse_named_color(s) {
        return Ok(c);
    }

    // Rgb(255, 255, 255)
    if let Some(rgb) = s.strip_prefix("Rgb(").and_then(|s| s.strip_suffix(")")) {
        let parts: Vec<_> = rgb.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().map_err(|_| "bad red value")?;
            let g = parts[1].parse::<u8>().map_err(|_| "bad green value")?;
            let b = parts[2].parse::<u8>().map_err(|_| "bad blue value")?;
            return Ok(Color::Rgb(r, g, b));
        }
    }

    // #rrggbb
    if let Some(hex) = s.strip_prefix('#')
        && hex.len() == 6
    {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "bad hex")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "bad hex")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "bad hex")?;
        return Ok(Color::Rgb(r, g, b));
    }

    Ok(Color::Magenta)
}

fn parse_named_color(name: &str) -> Result<Color, ()> {
    use Color::*;
    Ok(match name.cow_to_lowercase().as_ref() {
        "black" => Black,
        "red" => Red,
        "green" => Green,
        "yellow" => Yellow,
        "blue" => Blue,
        "magenta" => Magenta,
        "cyan" => Cyan,
        "gray" => Gray,
        "darkgray" => DarkGray,
        "lightred" => LightRed,
        "lightgreen" => LightGreen,
        "lightyellow" => LightYellow,
        "lightblue" => LightBlue,
        "lightmagenta" => LightMagenta,
        "lightcyan" => LightCyan,
        "white" => White,
        _ => return Err(()),
    })
}
