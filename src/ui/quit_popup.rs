use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    widgets::Clear,
};

use crate::{event::Action, tui::Event};

use super::{
    component::Component,
    theme::{themed_block, themed_button},
    util::centered_rect,
};

#[derive(Default)]
pub struct QuitPopup {
    selected: bool,
}

impl Component for QuitPopup {
    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Left => {
                self.selected = !self.selected;
                Ok(None)
            }
            Action::Right => {
                self.selected = !self.selected;
                Ok(None)
            }
            Action::Select => {
                if self.selected {
                    Ok(Some(Action::Quit))
                } else {
                    Ok(Some(Action::Back))
                }
            }
            _ => Ok(None), // No other actions handled here
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(40, 30, area);
        f.render_widget(Clear, popup_area);

        let padded_area = popup_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(padded_area);

        let button_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

        f.render_widget(themed_block("Quit Roxy"), popup_area);
        f.render_widget(themed_button("Yes", self.selected), button_layout[0]);
        f.render_widget(themed_button("No", !self.selected), button_layout[1]);

        Ok(())
    }
}
