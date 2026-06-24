use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};

use ratatui::{Frame, layout::Rect, text::Line};

use crate::{
    event::Action,
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
    pub(crate) current_tab: ConfigTab,
}

impl TabComponent {
    pub(crate) fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("ConfigTabs"),
            current_tab: ConfigTab::App,
        }
    }

    fn prev(&mut self) {
        self.current_tab = self.current_tab.prev();
    }

    fn next(&mut self) {
        self.current_tab = self.current_tab.next();
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
        Rect::default()
    }
}

impl Component for TabComponent {
    fn update(&mut self, action: Action) -> ActionResult {
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
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
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
}
