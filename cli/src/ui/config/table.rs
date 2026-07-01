use std::{collections::HashMap, fmt::Display, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, ratatui::layout::Rect};
use ratatui::{
    layout::{Constraint, Margin, Position},
    prelude::Frame,
    style::{Color, Stylize},
    text::Span,
    widgets::{Cell, Paragraph, Row, TableState},
};
use tracing::{debug, error, info};

use crate::{
    action::Action,
    config::{RoxyConfig, color::parse_color, manager::ConfigManager},
    ui::{
        config::{ConfigValue, EditableConfigField, tab::ConfigTab},
        framework::{
            color::color_is_light,
            component::{Component, DispatchCancellation, DispatchResult},
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
}

impl TableComponent {
    pub(crate) fn new(config_manager: ConfigManager, selected_tab: ConfigTab) -> Self {
        let config_rx = config_manager.rx.clone();
        let current_config = config_rx.borrow();
        let fields: HashMap<ConfigTab, Vec<EditableConfigField>> = (&*current_config).into();

        Self {
            focus: FocusFlag::new().with_name("TableComponent"),
            area: Rect::default(),
            config_manager,
            fields,
            selected_tab,
            table_state: TableState::default(),
            is_editing: false,
            input_buffer: String::new(),
        }
    }

    pub(crate) fn set_selected_tab(&mut self, tab: ConfigTab) {
        if self.selected_tab == tab {
            return;
        }
        self.selected_tab = tab;
        self.table_state.select(Some(0));
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

        if field.is_editing {
            field.is_editing = false;
            self.is_editing = false;

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

            self.update_config();
        } else {
            field.is_editing = true;
            self.is_editing = true;
            self.input_buffer = match &field.value {
                ConfigValue::String(s) => s.clone(),
                ConfigValue::U16(n) => n.to_string(),
                ConfigValue::Bool(b) => {
                    field.value = ConfigValue::Bool(!*b);
                    field.is_editing = false;
                    self.is_editing = false;
                    self.update_config();
                    return;
                }
                ConfigValue::Color(c) => c.to_string(),
                ConfigValue::Path(p) => p.display().to_string(),
            };
        }
    }

    fn update_config(&mut self) {
        debug!("Writing config");
        match RoxyConfig::try_from(self.fields.clone()) {
            Ok(cfg) => {
                if let Err(error) = self.config_manager.update(cfg) {
                    error!("Error writing config: '{error}'");
                } else {
                    debug!("Config written successfully");
                }
            }
            Err(error) => {
                error!("Error converting fields to config: '${error}'");
            }
        }
    }
}

impl Component for TableComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        if let Some(fields) = self.fields.get(&self.selected_tab) {
            let rows: Vec<Row> = fields
                .iter()
                .map(|field| {
                    let value_string: &dyn Display = if field.is_editing {
                        &self.input_buffer
                    } else {
                        match &field.value {
                            ConfigValue::Color(color) => color,
                            ConfigValue::String(string) => string,
                            ConfigValue::U16(u) => u,
                            ConfigValue::Bool(bool) => bool,
                            ConfigValue::Path(path) => &path.display(),
                        }
                    };

                    let mut value_span = Span::raw(value_string.to_string());
                    if let ConfigValue::Color(color) = field.value {
                        value_span = value_span.bg(color);
                        value_span = if color_is_light(&color) {
                            value_span.fg(Color::Black)
                        } else {
                            value_span.fg(Color::White)
                        }
                    }
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
                themed_table(
                    rows,
                    widths,
                    Some(self.selected_tab.title()),
                    self.focus.get(),
                ),
                area,
                &mut self.table_state,
            );
        } else {
            frame.render_widget(
                Paragraph::new("No fields").block(themed_block(
                    Some(self.selected_tab.title()),
                    self.focus.get(),
                )),
                area,
            );
        }
    }

    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        if !self.focus.get() {
            return Ok(());
        }

        // On select takes priority over other actions, we always handle it
        // to get in and out of edit mode.
        if Action::Select == *action {
            self.on_select();
            return DispatchCancellation::stop();
        }

        if self.is_editing() {
            return Ok(());
        }

        let mut result = DispatchCancellation::stop();
        match action {
            Action::Left => {
                self.table_state.select_previous_column();
            }
            Action::Right => {
                self.table_state.select_next_column();
            }
            Action::Up => {
                self.table_state.select_previous();
            }
            Action::Down => {
                self.table_state.select_next();
            }
            Action::PageUp => {
                self.table_state.scroll_up_by(self.area.height);
            }
            Action::PageDown => {
                self.table_state.scroll_down_by(self.area.height);
            }
            Action::Top => {
                self.table_state
                    .scroll_up_by(self.table_state.selected().unwrap_or(0) as u16);
            }
            Action::Bottom => {
                self.table_state.scroll_down_by(u16::MAX);
            }
            _ => result = Ok(()),
        }
        result
    }

    fn handle_key_event(&mut self, key: &KeyEvent) -> DispatchResult {
        if self.focus().get() && self.is_editing() {
            match key.code {
                KeyCode::Esc => {
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
            DispatchCancellation::stop()
        } else {
            Ok(())
        }
    }

    fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> DispatchResult {
        let position = Position {
            x: mouse.column,
            y: mouse.row,
        };
        if !self.area.contains(position) {
            return Ok(());
        }
        if !self.focus.get() {
            return DispatchCancellation::action(Action::FocusReq(self.focus.widget_id()));
        }

        if mouse.kind == MouseEventKind::ScrollDown {
            self.table_state.scroll_down_by(1);
            return Ok(());
        }
        if mouse.kind == MouseEventKind::ScrollUp {
            self.table_state.scroll_up_by(1);
            return Ok(());
        }

        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            // The actual area for the table consider margins
            let table_area = self.area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            });
            // Calculate the field to select based on the mouse click position and the current
            // scroll offset.
            let click_column = mouse.row - table_area.top();
            let scroll_offset = self.table_state.offset();
            let scroll_target = scroll_offset.saturating_add(click_column as usize);
            self.table_state.select(Some(scroll_target));
            return Ok(());
        }
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
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
