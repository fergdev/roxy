use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Position, Rect},
};

use crate::{action::Action, tui::TuiEvent};

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
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![]
    }
    fn area(&self) -> Rect;
    fn visible(&self) -> bool {
        self.area().width > 0 && self.area().height > 0
    }
    /// Handle tui events.
    fn dispatch_tui_events(&mut self, tui_event: TuiEvent) -> Result<Option<Action>> {
        for child in self.children() {
            if let Ok(Some(action)) = child.dispatch_tui_events(tui_event.clone()) {
                return Ok(Some(action));
            }
        }
        match tui_event {
            TuiEvent::Key(key) => match self.handle_key_event(&key) {
                KeyEventResult::Consumed => Ok(None),
                KeyEventResult::Action(action) => Ok(Some(action)),
                KeyEventResult::Ignored => Ok(None),
            },
            TuiEvent::Mouse(mouse) => {
                if self.area().contains(Position {
                    x: mouse.column,
                    y: mouse.row,
                }) {
                    self.handle_mouse_event(mouse)
                } else {
                    Ok(None)
                }
            }
            _ => self.handle_tui_event(tui_event),
        }
    }
    fn dispatch_action(&mut self, action: Action) -> ActionResult {
        for child in self.children() {
            let result = child.dispatch_action(action.clone());
            match result {
                ActionResult::Consumed => {
                    return result;
                }
                ActionResult::Action(action) => {
                    return ActionResult::Action(action);
                }
                ActionResult::Ignored => {}
            }
        }
        self.handle_action(action)
    }

    fn handle_tui_event(&mut self, _tui_event: TuiEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn handle_action(&mut self, _action: Action) -> ActionResult {
        ActionResult::Ignored
    }

    /// Handle crossterm key events.
    fn handle_key_event(&mut self, _key: &KeyEvent) -> KeyEventResult {
        KeyEventResult::Ignored
    }

    /// Handle crossterm mouse events.
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Draw to the frame within the given area.
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
