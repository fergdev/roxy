use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tracing::{debug, error};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Clear, Paragraph, Row, TableState},
};

use crate::{
    config::{ConfigManager, RoxyConfig, key_event_to_string, parse_color, parse_key_event},
    event::{Action, Mode},
};

use super::framework::{
    component::{ActionResult, Component, KeyEventResult},
    theme::{themed_table, themed_tabs, with_theme},
    util::centered_rect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConfigTab {
    App,
    Proxy,
    KeyBinds,
    Theme,
}

impl ConfigTab {
    fn all() -> &'static [ConfigTab] {
        &[Self::App, Self::Proxy, Self::KeyBinds, Self::Theme]
    }

    fn title(&self) -> &'static str {
        match self {
            Self::App => "App",
            Self::Proxy => "Proxy",
            Self::KeyBinds => "Keys",
            Self::Theme => "Theme",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> ConfigTab {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            Self::Theme // TODO: not great, but doesn't unwrap
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> ConfigTab {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            Self::App // TODO: not great, but doesn't unwrap
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Debug, Clone)]
enum ConfigValue {
    Bool(bool),
    U16(u16),
    String(String),
    Path(PathBuf),
    Color(Color),
}

#[derive(Debug, Clone)]
struct EditableConfigField {
    key: String,
    value: ConfigValue,
    editing: bool,
}

#[derive(Debug)]
pub struct ConfigEditor {
    focus: FocusFlag,
    config_manager: ConfigManager,
    curr_tab: ConfigTab,
    fields: HashMap<ConfigTab, Vec<EditableConfigField>>,
    table_state: TableState,
    input_buffer: String,
    is_editing: bool,
}

impl HasFocus for ConfigEditor {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl ConfigEditor {
    pub fn new(config_manager: ConfigManager) -> Self {
        let rx = config_manager.rx.clone();
        let cfg = rx.borrow();
        let fields: HashMap<ConfigTab, Vec<EditableConfigField>> = (&*cfg).into();

        Self {
            focus: FocusFlag::new().with_name("ConfigEditor"),
            config_manager,
            curr_tab: ConfigTab::App,
            fields,
            table_state: TableState::default(),
            input_buffer: String::new(),
            is_editing: false,
        }
    }

    fn on_up(&mut self) {
        if self.is_editing() {
            return;
        }
        self.table_state.select_previous()
    }

    fn on_down(&mut self) {
        if self.is_editing() {
            return;
        }
        self.table_state.select_next();
    }

    fn on_select(&mut self) {
        let Some(selected) = self.table_state.selected() else {
            return;
        };
        let new_val = self.input_buffer.trim().to_string(); // only immutable
        let fields = match self.fields.get_mut(&self.curr_tab) {
            Some(f) => f,
            None => {
                return;
            }
        };

        let field = &mut fields[selected];
        field.editing = !field.editing;
        if field.editing {
            field.editing = true;
            self.input_buffer = match &field.value {
                ConfigValue::String(s) => s.clone(),
                ConfigValue::U16(n) => n.to_string(),
                ConfigValue::Bool(b) => {
                    field.value = ConfigValue::Bool(!*b);
                    field.editing = false;
                    self.update_config();
                    return;
                }
                ConfigValue::Color(c) => c.to_string(),
                ConfigValue::Path(p) => p.display().to_string(),
            };
            self.is_editing = true;
        } else {
            field.editing = false;
            field.value = match &field.value {
                ConfigValue::String(_) => ConfigValue::String(new_val),
                ConfigValue::U16(_) => new_val
                    .parse()
                    .map(ConfigValue::U16)
                    .unwrap_or(field.value.clone()),
                ConfigValue::Bool(_) => ConfigValue::Bool(new_val.parse().unwrap_or(false)),
                ConfigValue::Path(_) => ConfigValue::Path(PathBuf::from(new_val)),
                ConfigValue::Color(_) => parse_color(&new_val)
                    .map(ConfigValue::Color)
                    .unwrap_or(field.value.clone()),
            };

            self.is_editing = false;
            self.update_config();
        }
    }

    fn update_config(&mut self) {
        debug!("Writing config");
        let cfg = RoxyConfig::try_from(self.fields.clone());
        match cfg {
            Ok(cfg) => {
                let _ = self.config_manager.update(cfg);
            }
            Err(e) => {
                error!("Error writing config: '${e}'");
            }
        }
    }

    pub fn on_key(&mut self, key: &KeyEvent) -> KeyEventResult {
        if self.is_editing() {
            match key.code {
                KeyCode::Esc => {
                    self.on_select();
                }
                KeyCode::Enter => {
                    self.on_select();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                _ => {}
            }
            KeyEventResult::Consumed
        } else {
            KeyEventResult::Ignored
        }
    }

    fn is_editing(&self) -> bool {
        self.is_editing
    }
}

impl From<&RoxyConfig> for HashMap<ConfigTab, Vec<EditableConfigField>> {
    fn from(cfg: &RoxyConfig) -> Self {
        let mut fields = HashMap::new();

        debug!("Setting confirm_quit");
        let app_fieldds = vec![
            EditableConfigField {
                key: "confirm_quit".into(),
                value: ConfigValue::Bool(cfg.app.confirm_quit),
                editing: false,
            },
            EditableConfigField {
                key: "data_dir".into(),
                value: ConfigValue::Path(cfg.app.data_dir.clone()),
                editing: false,
            },
            EditableConfigField {
                key: "config_dir".into(),
                value: ConfigValue::Path(cfg.app.config_dir.clone()),
                editing: false,
            },
        ];
        fields.insert(ConfigTab::App, app_fieldds);

        let proxy_fields = vec![
            EditableConfigField {
                key: "port".into(),
                value: ConfigValue::U16(cfg.app.proxy.port),
                editing: false,
            },
            EditableConfigField {
                key: "ca_cert_path".into(),
                value: match &cfg.app.proxy.ca_cert_path {
                    Some(path) => ConfigValue::Path(path.clone()),
                    None => ConfigValue::Path(PathBuf::new()),
                },
                editing: false,
            },
        ];

        fields.insert(ConfigTab::Proxy, proxy_fields);

        fields.insert(ConfigTab::Theme, gen_theme(cfg));

        let mut keybinds_fields = Vec::new();
        cfg.keybindings.iter().for_each(|(_section, binds)| {
            for (key, action) in binds {
                if let Some(key) = key.first() {
                    keybinds_fields.push(EditableConfigField {
                        key: action.to_string(), // TODO: yep should be vec
                        value: ConfigValue::String(key_event_to_string(key)), // TODO: yep should be vec
                        editing: false,
                    });
                }
            }
        });
        fields.insert(ConfigTab::KeyBinds, keybinds_fields);

        fields
    }
}

fn gen_theme(cfg: &RoxyConfig) -> Vec<EditableConfigField> {
    vec![
        EditableConfigField {
            key: "primary".into(),
            value: ConfigValue::Color(cfg.theme.colors.primary),
            editing: false,
        },
        EditableConfigField {
            key: "on_primary".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_primary),
            editing: false,
        },
        EditableConfigField {
            key: "secondary".into(),
            value: ConfigValue::Color(cfg.theme.colors.secondary),
            editing: false,
        },
        EditableConfigField {
            key: "on_secondary".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_secondary),
            editing: false,
        },
        EditableConfigField {
            key: "surface".into(),
            value: ConfigValue::Color(cfg.theme.colors.surface),
            editing: false,
        },
        EditableConfigField {
            key: "on_surface".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_surface),
            editing: false,
        },
        EditableConfigField {
            key: "background".into(),
            value: ConfigValue::Color(cfg.theme.colors.background),
            editing: false,
        },
        EditableConfigField {
            key: "on_background".into(),
            value: ConfigValue::Color(cfg.theme.colors.on_background),
            editing: false,
        },
        EditableConfigField {
            key: "outline".into(),
            value: ConfigValue::Color(cfg.theme.colors.outline),
            editing: false,
        },
        EditableConfigField {
            key: "outline_unfocused".into(),
            value: ConfigValue::Color(cfg.theme.colors.outline_unfocused),
            editing: false,
        },
        EditableConfigField {
            key: "error".into(),
            value: ConfigValue::Color(cfg.theme.colors.error),
            editing: false,
        },
        EditableConfigField {
            key: "info".into(),
            value: ConfigValue::Color(cfg.theme.colors.info),
            editing: false,
        },
        EditableConfigField {
            key: "warn".into(),
            value: ConfigValue::Color(cfg.theme.colors.warn),
            editing: false,
        },
        EditableConfigField {
            key: "debug".into(),
            value: ConfigValue::Color(cfg.theme.colors.debug),
            editing: false,
        },
        EditableConfigField {
            key: "trace".into(),
            value: ConfigValue::Color(cfg.theme.colors.trace),
            editing: false,
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
                            && let Ok(key_event) = parse_key_event(&s)
                        {
                            let action = Action::from_str(&field.key)
                                .map_err(|e| format!("Bad action: {e}"))?;
                            map.insert(vec![key_event], action);
                        }
                    }
                    config.keybindings.insert(Mode::Normal, map);
                }
            }
        }

        Ok(config)
    }
}

impl Component for ConfigEditor {
    fn update(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Up => {
                self.on_up();
                ActionResult::Consumed
            }
            Action::Down => {
                self.on_down();
                ActionResult::Consumed
            }
            Action::Left => {
                if !self.is_editing() {
                    self.curr_tab = self.curr_tab.prev();
                }
                ActionResult::Consumed
            }
            Action::Right => {
                if !self.is_editing() {
                    self.curr_tab = self.curr_tab.next();
                }
                ActionResult::Consumed
            }
            Action::Select => {
                self.on_select();
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);
        frame.render_widget(Clear, popup_area);

        let current_tab = self.curr_tab;

        let tab_titles: Vec<Line> = ConfigTab::all()
            .iter()
            .map(|t| Line::raw(t.title()))
            .collect();

        let tabs = themed_tabs(Some("Config"), tab_titles, current_tab.index(), true);

        let chunks =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(popup_area);

        frame.render_widget(tabs, chunks[0]);

        match self.fields.get(&current_tab) {
            Some(fields) => {
                let rows: Vec<Row> = fields
                    .iter()
                    .map(|field| {
                        let key = field.key.clone();

                        let value = if field.editing {
                            format!("(editing) {}", self.input_buffer)
                        } else {
                            match &field.value {
                                ConfigValue::Color(c) => format!("{c}"),
                                ConfigValue::String(s) => s.clone(),
                                ConfigValue::U16(n) => n.to_string(),
                                ConfigValue::Bool(b) => b.to_string(),
                                ConfigValue::Path(p) => p.display().to_string(),
                            }
                        };

                        let value_span = match &field.value {
                            ConfigValue::Color(color) => {
                                Span::styled(value, Style::default().fg(*color))
                            }
                            ConfigValue::String(s) => Span::raw(s.clone()),
                            _ => Span::raw(value),
                        };

                        let colors = with_theme(|t| t.colors.clone());
                        Row::new(vec![Cell::from(Span::raw(key)), Cell::from(value_span)]).style(
                            if field.editing {
                                Style::default()
                                    .bg(colors.surface)
                                    .fg(colors.primary)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().bg(colors.surface).fg(colors.on_surface)
                            },
                        )
                    })
                    .collect();

                let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];

                frame.render_stateful_widget(
                    themed_table(rows, widths, None, true),
                    chunks[1],
                    &mut self.table_state,
                );
            }
            None => {
                let empty_paragrah = Paragraph::new("No fields");
                frame.render_widget(empty_paragrah, area);
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: &KeyEvent) -> KeyEventResult {
        self.on_key(key)
    }
}
