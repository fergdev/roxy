use color_eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusFlag, HasFocus};

use ratatui::{
    Frame,
    layout::{Position, Rect},
    text::Line,
};
use tracing::debug;

use crate::{
    action::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_tabs,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ConfigTab {
    App,
    Proxy,
    KeyBinds,
    Theme,
}

impl ConfigTab {
    pub(crate) fn all() -> &'static [ConfigTab] {
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
            *all_tabs.last().unwrap_or(&Self::Theme)
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> ConfigTab {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.first().unwrap_or(&Self::App)
        } else {
            all_tabs[index + 1]
        }
    }
}

pub(crate) struct TabComponent {
    focus: FocusFlag,
    area: Rect,
    pub(crate) current_tab: ConfigTab,
    configtab_tx: tokio::sync::mpsc::UnboundedSender<ConfigTab>,
}

impl TabComponent {
    pub(crate) fn new(configtab_tx: tokio::sync::mpsc::UnboundedSender<ConfigTab>) -> Self {
        Self {
            focus: FocusFlag::new().with_name("ConfigTabs"),
            area: Rect::default(),
            current_tab: ConfigTab::App,
            configtab_tx,
        }
    }

    fn prev(&mut self) {
        self.current_tab = self.current_tab.prev();
        self.notify();
    }

    fn next(&mut self) {
        self.current_tab = self.current_tab.next();
        self.notify();
    }
    fn notify(&mut self) {
        self.configtab_tx
            .send(self.current_tab)
            .unwrap_or_else(|e| {
                debug!("Failed to send current tab: {}", e);
            });
    }
}

impl HasFocus for TabComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl Component for TabComponent {
    fn handle_action(&mut self, action: Action) -> ActionResult {
        if !self.focus.get() {
            return ActionResult::Ignored;
        }

        match action {
            Action::Left => {
                self.prev();
                ActionResult::Consumed
            }
            Action::Right => {
                self.next();
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<Option<Action>> {
        if !self.area.contains(Position {
            x: mouse_event.column,
            y: mouse_event.row,
        }) {
            return Ok(None);
        }
        if let MouseEventKind::Up(_) = mouse_event.kind {
            let component_up_x = mouse_event.column - self.area.x;
            // TODO: map tab_index by counting size of text
            // let len = ConfigTab::all().len() as u16;
            let tab_width = 7;

            // if a / 5 < ConfigTab::all().len() as u16 {
            let tab_index = (component_up_x / tab_width) as usize;
            if let Some(tab) = ConfigTab::all().get(tab_index) {
                self.current_tab = *tab;
                self.notify();
            }
            return Ok(Some(Action::FocusReq(self.focus.widget_id())));
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = area;
        let tab_titles: Vec<Line> = ConfigTab::all()
            .iter()
            .map(|t| Line::raw(t.title()))
            .collect();

        let tabs = themed_tabs(
            Some("Config"),
            tab_titles,
            self.current_tab.index(),
            self.focus.get(),
        );
        frame.render_widget(tabs, area);
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }
}
