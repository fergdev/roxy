mod fields;
mod tab;
mod table;

use color_eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::Clear,
};

use crate::{
    action::Action,
    config::manager::ConfigManager,
    tui::TuiEvent,
    ui::{
        config::{tab::ConfigTab, table::TableComponent},
        framework::tab::TabComponent,
    },
};

use super::framework::{component::Component, util::centered_rect};

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
    area: Rect,
    tab_component: TabComponent,
    table_component: TableComponent,
}

impl ConfigEditor {
    pub fn new(config_manager: ConfigManager) -> Self {
        Self {
            focus: FocusFlag::new().with_name("ConfigEditor"),
            area: Rect::default(),
            tab_component: TabComponent::new(
                "Confg Editor".to_string(),
                ConfigTab::all()
                    .iter()
                    .map(|t| t.title().to_string())
                    .collect::<Vec<_>>(),
            ),
            table_component: TableComponent::new(config_manager, ConfigTab::App),
        }
    }

    pub fn shown(&mut self) {
        self.tab_component.focus().set(true);
    }
}

impl Component for ConfigEditor {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.tab_component, &mut self.table_component]
    }

    fn handle_tui_event(&mut self, tui_event: TuiEvent) -> Result<Option<Action>> {
        if tui_event == TuiEvent::Render {
            self.table_component
                .set_selected_tab(ConfigTab::all()[self.tab_component.current_tab]);
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);
        self.area = popup_area;
        frame.render_widget(Clear, popup_area);

        let chunks =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(popup_area);

        self.tab_component.render(frame, chunks[0])?;
        self.table_component.render(frame, chunks[1])?;

        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
impl HasFocus for ConfigEditor {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.tab_component);
        builder.widget(&self.table_component);
        builder.end(tag);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}
