use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{event::Action, tui::Event};

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
    fn register_action_handler(&mut self, _tx: UnboundedSender<Action>) -> Result<()> {
        Ok(())
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }

    fn focus(&mut self) {}

    fn unfocus(&mut self) {}

    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        let action = match event {
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }
    fn handle_key_event(&mut self, _key: &KeyEvent) -> KeyEventResult {
        KeyEventResult::Ignored
    }
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> ActionResult {
        ActionResult::Ignored
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
