use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};

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
    fn area(&self) -> Rect {
        Rect::default()
    }
    fn visible(&self) -> bool {
        self.area() != Rect::default()
    }
    /// Handle tui events.
    fn handle_tui_event(&mut self, _tui_event: TuiEvent) -> Result<Option<Action>> {
        Ok(None)
    }
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
            TuiEvent::Mouse(mouse) => self.handle_mouse_event(mouse),
            _ => self.handle_tui_event(tui_event),
        }
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

    fn dispatch_action(&mut self, action: Action) -> ActionResult {
        for child in self.children() {
            let r = child.dispatch_action(action.clone());
            match r {
                ActionResult::Consumed => {
                    return r;
                }
                ActionResult::Action(action) => {
                    return ActionResult::Action(action);
                }
                ActionResult::Ignored => {}
            }
        }
        self.handle_action(action)
    }

    /// Draw to the frame within the given area.
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
