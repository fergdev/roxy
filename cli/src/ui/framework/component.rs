use crossterm::event::{KeyEvent, MouseEvent};
use rat_focus::FocusFlag;
use ratatui::{
    Frame,
    layout::{Position, Rect},
};

use crate::{action::Action, tui::TuiEvent};

pub enum DispatchCancellation {
    Stop,
    Bubble(Action),
}

impl DispatchCancellation {
    #[inline]
    pub fn stop() -> DispatchResult {
        Err(DispatchCancellation::Stop)
    }
    #[inline]
    pub fn action(action: Action) -> DispatchResult {
        Err(DispatchCancellation::Bubble(action))
    }
}

pub type DispatchResult = Result<(), DispatchCancellation>;

pub trait Component {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![]
    }
    fn area(&self) -> Rect;
    fn visible(&self) -> bool {
        self.area().width > 0 && self.area().height > 0
    }
    fn focus(&mut self) -> &mut FocusFlag;
    /// Handle tui events.
    fn dispatch_tui_events(&mut self, tui_event: &TuiEvent) -> DispatchResult {
        for child in self.children() {
            child.dispatch_tui_events(tui_event)?;
        }
        match tui_event {
            TuiEvent::Key(key) => self.handle_key_event(key),
            TuiEvent::Mouse(mouse) => {
                if self.area().contains(Position {
                    x: mouse.column,
                    y: mouse.row,
                }) {
                    self.handle_mouse_event(mouse)
                } else {
                    Ok(())
                }
            }
            _ => self.handle_tui_event(tui_event),
        }
    }
    fn dispatch_action(&mut self, action: &Action) -> DispatchResult {
        for child in self.children() {
            child.dispatch_action(action)?;
        }
        if self.focus().get() {
            // debug!(
            //     "ActionHandle[117]: {}:{}: name={:#?}",
            //     file!(),
            //     line!(),
            //     self.focus().name()
            // );
            self.handle_action(action)
        } else {
            // debug!(
            //     "ActionIgnored[117]: {}:{}: name={:#?}",
            //     file!(),
            //     line!(),
            //     self.focus().name()
            // );

            Ok(())
        }
    }

    fn handle_tui_event(&mut self, _tui_event: &TuiEvent) -> DispatchResult {
        Ok(())
    }

    /// Handle actions, only invoked when the component has focus.
    fn handle_action(&mut self, _action: &Action) -> DispatchResult {
        Ok(())
    }

    /// Handle crossterm key events.
    fn handle_key_event(&mut self, _key: &KeyEvent) -> DispatchResult {
        Ok(())
    }

    /// Handle crossterm mouse events, only invoked when the mouse event is within the component's
    /// area.
    fn handle_mouse_event(&mut self, _mouse: &MouseEvent) -> DispatchResult {
        Ok(())
    }

    /// Draw to the frame within the given area.
    fn render(&mut self, frame: &mut Frame, area: Rect);
}
