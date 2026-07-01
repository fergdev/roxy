use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::Clear,
};
use tokio::sync::{mpsc::UnboundedSender, watch};

use crate::{
    action::Action,
    ui::framework::button_group::{ButtonGroup, ButtonGroupEvent},
};

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

impl QuitPopup {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        let (tx, rx) = watch::channel(ButtonGroupEvent::Hovered(0));

        tokio::spawn(async move {
            let mut rx = rx;
            while rx.changed().await.is_ok() {
                if let ButtonGroupEvent::Selected(index) = *rx.borrow() {
                    if index == 0 {
                        let _ = action_tx.send(Action::Quit);
                    } else {
                        let _ = action_tx.send(Action::Back);
                    }
                }
            }
        });

        Self {
            focus: FocusFlag::new().with_name("QuitPopup"),
            area: Rect::default(),
            button_group: ButtonGroup::new(
                "QuitPopup",
                vec!["Yes".to_string(), "No".to_string()],
                1,
                tx,
            ),
        }
    }

    pub fn reset(&mut self) {
        self.button_group.selected_index = 1;
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

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = centered_rect_abs(30, 3, area);
        frame.render_widget(Clear, self.area);

        let padded_area = self.area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        frame.render_widget(themed_block(Some("Quit Roxy"), true), self.area);
        self.button_group.render(frame, padded_area);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.button_group.focus
    }
}
