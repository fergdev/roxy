use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{event::Action, tui::Event};

pub trait Component {
    fn register_action_handler(&mut self, _tx: UnboundedSender<Action>) -> Result<()> {
        Ok(())
    }
    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }
    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        let action = match event {
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }
    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
