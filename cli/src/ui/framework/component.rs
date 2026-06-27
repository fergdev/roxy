use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};

use crate::{event::Action, tui::TuiEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum KeyEventResult {
    Consumed,
    Action(Action),
    Ignored,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult {
    Consumed,
    Action(Action),
    Ignored,
}

pub trait Component {
    /// Handle tui events.
    fn handle_tui_event(&mut self, tui_event: TuiEvent) -> Result<Option<Action>> {
        let action = match tui_event {
            TuiEvent::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }

    /// Handle crossterm key events.
    fn handle_key_event(&mut self, _key: &KeyEvent) -> KeyEventResult {
        KeyEventResult::Ignored
    }

    /// Handle crossterm mouse events.
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn handle_action(&mut self, _action: Action) -> ActionResult {
        ActionResult::Ignored
    }

    /// Draw to the frame within the given area.
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
