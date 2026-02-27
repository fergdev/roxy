use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    widgets::Clear,
};

use crate::event::Action;

use super::framework::{
    component::{ActionResult, Component},
    theme::{themed_block, themed_button},
    util::centered_rect_abs,
};

#[derive(Default)]
pub struct QuitPopup {
    focus: FocusFlag,
    selected: bool,
}

impl HasFocus for QuitPopup {
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

impl QuitPopup {
    pub fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("QuitPopup"),
            selected: false,
        }
    }

    pub fn reset(&mut self) {
        self.selected = false;
    }
}

impl Component for QuitPopup {
    fn update(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Left => {
                self.selected = !self.selected;
                ActionResult::Consumed
            }
            Action::Right => {
                self.selected = !self.selected;
                ActionResult::Consumed
            }
            Action::Select => {
                if self.selected {
                    ActionResult::Action(Action::Quit)
                } else {
                    ActionResult::Action(Action::Back)
                }
            }
            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect_abs(30, 4, area);
        f.render_widget(Clear, popup_area);

        let padded_area = popup_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let layout =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(padded_area);

        let button_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

        f.render_widget(themed_block(Some("Quit Roxy"), true), popup_area);
        f.render_widget(themed_button("Yes", self.selected), button_layout[0]);
        f.render_widget(themed_button("No", !self.selected), button_layout[1]);

        Ok(())
    }
}
