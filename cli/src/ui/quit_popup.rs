use color_eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::Clear,
};

use crate::{action::Action, ui::framework::button_group::ButtonGroup};

use super::framework::{
    component::{ActionResult, Component},
    theme::themed_block,
    util::centered_rect_abs,
};

#[derive(Debug)]
pub struct QuitPopup {
    focus: FocusFlag,
    area: Rect,

    pub(crate) button_group: ButtonGroup,
}

impl HasFocus for QuitPopup {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.leaf_widget(&self.button_group);
        builder.end(tag);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl Default for QuitPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl QuitPopup {
    pub fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("QuitPopup"),
            area: Rect::default(),
            button_group: ButtonGroup::new("QuitPopup", vec!["Yes".to_string(), "No".to_string()]),
        }
    }

    pub fn reset(&mut self) {
        self.button_group.selected_index = 0;
        self.button_group.focus().set(true);
    }
}

impl Component for QuitPopup {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.button_group]
    }
    fn handle_action(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Select => {
                if self.button_group.selected_index == 0 {
                    ActionResult::Action(Action::Quit)
                } else {
                    ActionResult::Action(Action::Back)
                }
            }
            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = centered_rect_abs(30, 3, area);
        frame.render_widget(Clear, self.area);

        let padded_area = self.area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        frame.render_widget(themed_block(Some("Quit Roxy"), true), self.area);
        self.button_group.render(frame, padded_area)?;

        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.button_group.focus
    }
}
