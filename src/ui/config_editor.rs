use color_eyre::Result;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::config::Config;

use super::{component::Component, util::centered_rect};

#[derive(Debug, Clone)]
pub enum ConfigValue {
    Bool(bool),
    U16(u16),
    String(String),
    Path(PathBuf),
    None,
}

#[derive(Debug, Clone)]
pub struct EditableConfigField {
    pub key: String,
    pub section: String,
    pub value: ConfigValue,
    pub editing: bool,
}

#[derive(Debug)]
pub struct ConfigEditor {
    pub fields: Vec<EditableConfigField>,
    pub selected: usize,
    pub input_buffer: String,
}

impl ConfigEditor {
    pub fn new() -> Self {
        let fields = vec![
            EditableConfigField {
                key: "data_dir".into(),
                section: "app".into(),
                value: ConfigValue::Path(PathBuf::from("~/.roxy/data")),
                editing: false,
            },
            EditableConfigField {
                key: "enabled".into(),
                section: "proxy".into(),
                value: ConfigValue::Bool(true),
                editing: false,
            },
            EditableConfigField {
                key: "port".into(),
                section: "proxy".into(),
                value: ConfigValue::U16(6969),
                editing: false,
            },
        ];
        Self {
            fields,
            selected: 0,
            input_buffer: String::new(),
        }
    }

    pub fn update_config(&self, cfg: &mut Config) {
        for field in &self.fields {
            match (field.section.as_str(), field.key.as_str(), &field.value) {
                ("app", "data_dir", ConfigValue::Path(p)) => cfg.app.data_dir = p.clone(),
                ("app", "config_dir", ConfigValue::Path(p)) => cfg.app.config_dir = p.clone(),
                ("app", "theme", ConfigValue::String(s)) => cfg.app.theme = Some(s.clone()),
                ("proxy", "enabled", ConfigValue::Bool(b)) => {
                    cfg.app.proxy.get_or_insert(Default::default()).enabled = *b
                }
                ("proxy", "port", ConfigValue::U16(p)) => {
                    cfg.app.proxy.get_or_insert(Default::default()).port = *p
                }
                ("proxy", "ca_cert_path", ConfigValue::Path(p)) => {
                    cfg.app.proxy.get_or_insert(Default::default()).ca_cert_path = Some(p.clone())
                }
                _ => {}
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                if self.selected + 1 < self.fields.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                let field = &mut self.fields[self.selected];
                field.editing = !field.editing;
                if field.editing {
                    self.input_buffer = match &field.value {
                        ConfigValue::String(s) => s.clone(),
                        ConfigValue::U16(n) => n.to_string(),
                        ConfigValue::Bool(b) => b.to_string(),
                        ConfigValue::Path(p) => p.display().to_string(),
                        ConfigValue::None => String::new(),
                    };
                } else {
                    // Apply edit
                    let new_val = self.input_buffer.trim();
                    field.value = match &field.value {
                        ConfigValue::String(_) => ConfigValue::String(new_val.into()),
                        ConfigValue::U16(_) => new_val
                            .parse()
                            .map(ConfigValue::U16)
                            .unwrap_or(ConfigValue::U16(0)),
                        ConfigValue::Bool(_) => ConfigValue::Bool(new_val.parse().unwrap_or(false)),
                        ConfigValue::Path(_) => ConfigValue::Path(PathBuf::from(new_val)),
                        ConfigValue::None => ConfigValue::String(new_val.into()),
                    };
                }
            }
            KeyCode::Char(c) if self.fields[self.selected].editing => {
                self.input_buffer.push(c);
            }
            KeyCode::Backspace if self.fields[self.selected].editing => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }
}

impl From<&Config> for Vec<EditableConfigField> {
    fn from(cfg: &Config) -> Self {
        let mut fields = Vec::new();

        // App section
        fields.push(EditableConfigField {
            key: "data_dir".into(),
            section: "app".into(),
            value: ConfigValue::Path(cfg.app.data_dir.clone()),
            editing: false,
        });

        fields.push(EditableConfigField {
            key: "config_dir".into(),
            section: "app".into(),
            value: ConfigValue::Path(cfg.app.config_dir.clone()),
            editing: false,
        });

        if let Some(proxy) = &cfg.app.proxy {
            fields.push(EditableConfigField {
                key: "enabled".into(),
                section: "proxy".into(),
                value: ConfigValue::Bool(proxy.enabled),
                editing: false,
            });

            fields.push(EditableConfigField {
                key: "port".into(),
                section: "proxy".into(),
                value: ConfigValue::U16(proxy.port),
                editing: false,
            });

            fields.push(EditableConfigField {
                key: "ca_cert_path".into(),
                section: "proxy".into(),
                value: match &proxy.ca_cert_path {
                    Some(path) => ConfigValue::Path(path.clone()),
                    None => ConfigValue::None,
                },
                editing: false,
            });
        }

        if let Some(theme) = &cfg.app.theme {
            fields.push(EditableConfigField {
                key: "theme".into(),
                section: "app".into(),
                value: ConfigValue::String(theme.clone()),
                editing: false,
            });
        }

        fields
    }
}

impl Component for ConfigEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);
        let items: Vec<ListItem> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let value = if field.editing {
                    format!("(editing) {}", self.input_buffer)
                } else {
                    match &field.value {
                        ConfigValue::String(s) => s.clone(),
                        ConfigValue::U16(n) => n.to_string(),
                        ConfigValue::Bool(b) => b.to_string(),
                        ConfigValue::Path(p) => p.display().to_string(),
                        ConfigValue::None => "-".to_string(),
                    }
                };

                let content = format!(
                    "{}.{}: {}{}",
                    field.section,
                    field.key,
                    if i == self.selected { "> " } else { "  " },
                    value
                );
                ListItem::new(Line::raw(content))
            })
            .collect();

        frame.render_widget(Clear, popup_area);
        frame.render_widget(
            List::new(items)
                .block(Block::default().title("Edit Config").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
            popup_area,
        );
        Ok(())
    }
}
