mod fields;
mod tab;
mod table;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::Clear,
};

use crate::{
    config::manager::ConfigManager,
    event::Action,
    ui::config::{tab::TabComponent, table::TableComponent},
};

use super::framework::{
    component::{ActionResult, Component, KeyEventResult},
    util::centered_rect,
};

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
    is_editing: bool,
}

pub struct ConfigEditor {
    focus: FocusFlag,
    tab_component: TabComponent,
    table_component: TableComponent,
}

impl HasFocus for ConfigEditor {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.tab_component);
        builder.widget(&self.table_component);
        builder.end(tag);
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
        Self {
            focus: FocusFlag::new().with_name("ConfigEditor"),
            tab_component: TabComponent::new(),
            table_component: TableComponent::new(config_manager),
        }
    }

    pub fn shown(&mut self) {
        self.tab_component.focus().get();
        self.update_fields();
    }

    fn update_fields(&mut self) {
        self.table_component
            .set_config_tab(&self.tab_component.current_tab);
    }
}

impl Component for ConfigEditor {
    fn update(&mut self, action: Action) -> ActionResult {
        if self.tab_component.update(action.clone()) == ActionResult::Consumed {
            self.update_fields();
            return ActionResult::Consumed;
        }
        self.table_component.update(action)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);
        frame.render_widget(Clear, popup_area);

        let chunks =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(popup_area);

        self.tab_component.render(frame, chunks[0])?;
        self.table_component.render(frame, chunks[1])?;

        Ok(())
    }
    fn handle_key_event(&mut self, key: &KeyEvent) -> KeyEventResult {
        self.table_component.handle_key_event(key)
    }
}
