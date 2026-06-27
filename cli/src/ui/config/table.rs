use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, ratatui::layout::Rect};
use ratatui::{
    layout::{Constraint, Position},
    prelude::Frame,
    style::Stylize,
    text::Span,
    widgets::{Cell, Paragraph, Row, TableState},
};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

use crate::{
    action::Action,
    config::{RoxyConfig, color::parse_color, manager::ConfigManager},
    tui::TuiEvent,
    ui::{
        config::{ConfigValue, EditableConfigField, tab::ConfigTab},
        framework::{
            component::{ActionResult, Component, KeyEventResult},
            theme::{themed_block, themed_table},
        },
    },
};

pub(crate) struct TableComponent {
    focus: FocusFlag,
    area: Rect,
    config_manager: ConfigManager,
    fields: HashMap<ConfigTab, Vec<EditableConfigField>>,
    selected_tab: ConfigTab,
    table_state: TableState,
    is_editing: bool,
    input_buffer: String,
    on_change: UnboundedReceiver<ConfigTab>,
}

impl TableComponent {
    pub(crate) fn new(
        config_manager: ConfigManager,
        on_change: UnboundedReceiver<ConfigTab>,
    ) -> Self {
        let config_rx = config_manager.rx.clone();
        let current_config = config_rx.borrow();
        let fields: HashMap<ConfigTab, Vec<EditableConfigField>> = (&*current_config).into();

        Self {
            focus: FocusFlag::new().with_name("TableComponent"),
            area: Rect::default(),
            config_manager,
            fields,
            selected_tab: ConfigTab::App,
            table_state: TableState::default(),
            is_editing: false,
            input_buffer: String::new(),
            on_change,
        }
    }

    fn is_editing(&self) -> bool {
        self.is_editing
    }

    fn on_select(&mut self) {
        info!("On select");
        let Some(selected_field_index) = self.table_state.selected() else {
            return;
        };
        info!("Selected {selected_field_index}");
        let new_val = self.input_buffer.trim().to_string();
        let fields = match self.fields.get_mut(&self.selected_tab) {
            Some(r) => r,
            None => {
                error!("Missing tab {:?}", self.selected_tab);
                return;
            }
        };
        let field = &mut fields[selected_field_index];
        field.is_editing = !field.is_editing;

        if field.is_editing {
            field.is_editing = true;
            self.input_buffer = match &field.value {
                ConfigValue::String(s) => s.clone(),
                ConfigValue::U16(n) => n.to_string(),
                ConfigValue::Bool(b) => {
                    field.value = ConfigValue::Bool(!*b);
                    field.is_editing = false;
                    self.update_config();
                    return;
                }
                ConfigValue::Color(c) => c.to_string(),
                ConfigValue::Path(p) => p.display().to_string(),
            };
            self.is_editing = true;
        } else {
            field.is_editing = false;
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

    // fn delete(&mut self) -> bool {
    //     let Some(selected_field_index) = self.table_state.selected() else {
    //         return false;
    //     };
    //
    //     // Delete keybinding
    //     if self.tab_component.current_tab == ConfigTab::KeyBinds {
    //         let fields = match self.fields.get_mut(&self.tab_component.current_tab) {
    //             Some(f) => f,
    //             None => {
    //                 return false;
    //             }
    //         };
    //
    //         fields.remove(selected_field_index);
    //         self.update_config();
    //         return true;
    //     }
    //
    //     false
    // }
    //
    // fn add(&mut self) -> bool {
    //     let Some(selected_field_index) = self.table_state.selected() else {
    //         return false;
    //     };
    //
    //     // Add keybinding
    //     if self.tab_component.current_tab == ConfigTab::KeyBinds {
    //         let fields = match self.fields.get_mut(&self.tab_component.current_tab) {
    //             Some(f) => f,
    //             None => {
    //                 return false;
    //             }
    //         };
    //
    //         let ecf = EditableConfigField {
    //             key: "Add".to_string(),
    //             value: ConfigValue::String("u".to_string()),
    //             is_editing: false,
    //         };
    //         fields.insert(selected_field_index, ecf);
    //         self.update_config();
    //         return true;
    //     }
    //     todo!()
    // }
}

impl Component for TableComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = area;
        if let Some(fields) = self.fields.get(&self.selected_tab) {
            let rows: Vec<Row> = fields
                .iter()
                .map(|field| {
                    let value_string = if field.is_editing {
                        self.input_buffer.to_owned()
                    } else {
                        match &field.value {
                            ConfigValue::Color(color) => format!("{color}"),
                            ConfigValue::String(string) => string.clone(),
                            ConfigValue::U16(u) => u.to_string(),
                            ConfigValue::Bool(bool) => bool.to_string(),
                            ConfigValue::Path(path) => path.display().to_string(),
                        }
                    };

                    let mut value_span = Span::raw(value_string);
                    if field.is_editing {
                        value_span = value_span.underlined()
                    }
                    Row::new(vec![
                        Cell::from(Span::raw(&field.key)),
                        Cell::from(value_span),
                    ])
                })
                .collect();

            let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];

            frame.render_stateful_widget(
                themed_table(rows, widths, None, self.focus.get()),
                area,
                &mut self.table_state,
            );
        } else {
            frame.render_widget(
                Paragraph::new("No fields").block(themed_block(None, self.focus.get())),
                area,
            );
        }
        Ok(())
    }

    fn handle_tui_event(&mut self, tui_event: crate::tui::TuiEvent) -> Result<Option<Action>> {
        // On render we check for new config_tab and update.
        if tui_event == TuiEvent::Render
            && let Ok(config) = self.on_change.try_recv()
        {
            self.selected_tab = config;
        }

        match tui_event {
            TuiEvent::Mouse(mouse) => match self.handle_mouse_event(mouse) {
                Ok(Some(action)) => return Ok(Some(action)),
                Ok(None) => {}
                Err(error) => return Err(error),
            },
            TuiEvent::Key(key) => {
                let result = self.handle_key_event(&key);
                if result == KeyEventResult::Consumed {
                    return Ok(None);
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
        if !self.focus.get() {
            return ActionResult::Ignored;
        }
        match action {
            Action::Select => {
                self.on_select();
                ActionResult::Consumed
            }
            Action::Left => {
                if self.is_editing() {
                    ActionResult::Ignored
                } else {
                    self.table_state.select_previous_column();
                    ActionResult::Consumed
                }
            }
            Action::Right => {
                if self.is_editing() {
                    ActionResult::Ignored
                } else {
                    self.table_state.select_next_column();
                    ActionResult::Consumed
                }
            }
            Action::Up => {
                if self.is_editing() {
                    ActionResult::Ignored
                } else {
                    self.table_state.select_previous();
                    ActionResult::Consumed
                }
            }
            Action::Down => {
                if self.is_editing() {
                    ActionResult::Ignored
                } else {
                    self.table_state.select_next();
                    ActionResult::Consumed
                }
            }
            _ => ActionResult::Ignored,
        }
    }

    fn handle_key_event(&mut self, key: &KeyEvent) -> KeyEventResult {
        if self.is_editing() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
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

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        if !self.area.contains(Position {
            x: mouse.column,
            y: mouse.row,
        }) {
            return Ok(None);
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            return Ok(Some(Action::FocusReq(self.focus.widget_id())));
        }
        Ok(None)
    }
}
impl HasFocus for TableComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(&self.focus);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}
