use color_eyre::Result;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tracing::debug;

use crate::{
    config::{
        RoxyConfig,
        keys::{key_sequence_to_string, parse_key_sequence},
    },
    event::{Action, KeyInputMode},
    ui::config::{ConfigValue, EditableConfigField, tab::ConfigTab},
};

impl From<&RoxyConfig> for HashMap<ConfigTab, Vec<EditableConfigField>> {
    fn from(roxy_config: &RoxyConfig) -> Self {
        let mut fields = HashMap::new();
        fields.insert(ConfigTab::App, gen_app(roxy_config));
        fields.insert(ConfigTab::Proxy, gen_proxy(roxy_config));
        fields.insert(ConfigTab::Theme, gen_theme(roxy_config));
        fields.insert(ConfigTab::KeyBinds, gen_keys(roxy_config));
        fields
    }
}
fn gen_keys(roxy_config: &RoxyConfig) -> Vec<EditableConfigField> {
    let mut keybinds_fields = Vec::new();
    roxy_config
        .keybindings
        .iter()
        .for_each(|(_section, binds)| {
            for (key_sequence, action) in binds {
                let display_str = key_sequence_to_string(key_sequence);
                keybinds_fields.push(EditableConfigField {
                    key: action.to_string(),
                    value: ConfigValue::String(display_str),
                    is_editing: false,
                });
            }
        });
    keybinds_fields
}

fn gen_app(roxy_config: &RoxyConfig) -> Vec<EditableConfigField> {
    vec![
        EditableConfigField {
            key: "confirm_quit".into(),
            value: ConfigValue::Bool(roxy_config.app.confirm_quit),
            is_editing: false,
        },
        EditableConfigField {
            key: "data_dir".into(),
            value: ConfigValue::Path(roxy_config.app.data_dir.clone()),
            is_editing: false,
        },
        EditableConfigField {
            key: "config_dir".into(),
            value: ConfigValue::Path(roxy_config.app.config_dir.clone()),
            is_editing: false,
        },
    ]
}

fn gen_proxy(roxy_config: &RoxyConfig) -> Vec<EditableConfigField> {
    vec![
        EditableConfigField {
            key: "port".into(),
            value: ConfigValue::U16(roxy_config.app.proxy.port),
            is_editing: false,
        },
        EditableConfigField {
            key: "ca_cert_path".into(),
            value: match &roxy_config.app.proxy.ca_cert_path {
                Some(path) => ConfigValue::Path(path.clone()),
                None => ConfigValue::Path(PathBuf::new()),
            },
            is_editing: false,
        },
    ]
}

fn gen_theme(cfg: &RoxyConfig) -> Vec<EditableConfigField> {
    vec![
        EditableConfigField {
            key: "primary".into(),
            value: ConfigValue::Color(cfg.theme.colors.primary),
            is_editing: false,
        },
        EditableConfigField {
            key: "on_primary".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_primary),
            is_editing: false,
        },
        EditableConfigField {
            key: "secondary".into(),
            value: ConfigValue::Color(cfg.theme.colors.secondary),
            is_editing: false,
        },
        EditableConfigField {
            key: "on_secondary".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_secondary),
            is_editing: false,
        },
        EditableConfigField {
            key: "surface".into(),
            value: ConfigValue::Color(cfg.theme.colors.surface),
            is_editing: false,
        },
        EditableConfigField {
            key: "on_surface".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_surface),
            is_editing: false,
        },
        EditableConfigField {
            key: "background".into(),
            value: ConfigValue::Color(cfg.theme.colors.background),
            is_editing: false,
        },
        EditableConfigField {
            key: "on_background".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_background),
            is_editing: false,
        },
        EditableConfigField {
            key: "outline".into(),
            value: ConfigValue::Color(cfg.theme.colors.outline),
            is_editing: false,
        },
        EditableConfigField {
            key: "outline_unfocused".into(),
            value: ConfigValue::Color(cfg.theme.colors.outline_unfocused),
            is_editing: false,
        },
        EditableConfigField {
            key: "error".into(),
            value: ConfigValue::Color(cfg.theme.colors.error),
            is_editing: false,
        },
        EditableConfigField {
            key: "info".into(),
            value: ConfigValue::Color(cfg.theme.colors.info),
            is_editing: false,
        },
        EditableConfigField {
            key: "warn".into(),
            value: ConfigValue::Color(cfg.theme.colors.warn),
            is_editing: false,
        },
        EditableConfigField {
            key: "debug".into(),
            value: ConfigValue::Color(cfg.theme.colors.debug),
            is_editing: false,
        },
        EditableConfigField {
            key: "trace".into(),
            value: ConfigValue::Color(cfg.theme.colors.trace),
            is_editing: false,
        },
    ]
}

impl TryFrom<HashMap<ConfigTab, Vec<EditableConfigField>>> for RoxyConfig {
    type Error = String;

    fn try_from(map: HashMap<ConfigTab, Vec<EditableConfigField>>) -> Result<Self, Self::Error> {
        let mut config = RoxyConfig::default();
        debug!("Try from map");

        for (tab, fields) in map {
            match tab {
                ConfigTab::App => {
                    for field in fields {
                        match field.key.as_str() {
                            "confirm_quit" => {
                                debug!("Writing confirm quit");
                                if let ConfigValue::Bool(p) = field.value {
                                    config.app.confirm_quit = p;
                                }
                            }
                            "data_dir" => {
                                if let ConfigValue::Path(p) = field.value.clone() {
                                    config.app.data_dir = p;
                                }
                            }
                            "config_dir" => {
                                if let ConfigValue::Path(p) = field.value.clone() {
                                    config.app.config_dir = p;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                ConfigTab::Proxy => {
                    for field in fields {
                        match field.key.as_str() {
                            "port" => {
                                if let ConfigValue::U16(n) = field.value {
                                    config.app.proxy.port = n;
                                }
                            }
                            "ca_cert_path" => {
                                if let ConfigValue::Path(p) = field.value.clone() {
                                    config.app.proxy.ca_cert_path = Some(p);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                ConfigTab::Theme => {
                    for field in fields {
                        let color = match field.value {
                            ConfigValue::Color(c) => c,
                            _ => continue,
                        };

                        match field.key.as_str() {
                            "primary" => config.theme.colors.primary = color,
                            "on_primary" => config.theme.colors.on_primary = color,
                            "secondary" => config.theme.colors.secondary = color,
                            "on_secondary" => config.theme.colors.on_secondary = color,
                            "surface" => config.theme.colors.surface = color,
                            "on_surface" => config.theme.colors.on_surface = color,
                            "background" => config.theme.colors.background = color,
                            "on_background" => config.theme.colors.on_background = color,
                            "outline" => config.theme.colors.outline = color,
                            "error" => config.theme.colors.error = color,
                            "success" => config.theme.colors.success = color,
                            "warn" => config.theme.colors.warn = color,
                            "info" => config.theme.colors.info = color,
                            "debug" => config.theme.colors.debug = color,
                            "trace" => config.theme.colors.trace = color,
                            _ => {}
                        }
                    }
                }

                ConfigTab::KeyBinds => {
                    let mut map = HashMap::new();
                    for field in fields {
                        if let ConfigValue::String(s) = field.value.clone()
                            && let Ok(key_event) = parse_key_sequence(&s)
                        {
                            let action = Action::from_str(&field.key)
                                .map_err(|e| format!("Bad action: {e}"))?;
                            map.insert(key_event, action);
                        }
                    }
                    config.keybindings.insert(KeyInputMode::Normal, map);
                }
            }
        }

        Ok(config)
    }
}
