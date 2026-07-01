use color_eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Offset, Rect},
};
use tokio::sync::watch::Sender;

use crate::action::Action;

use super::{
    component::{ActionResult, Component},
    theme::themed_button,
};
#[derive(Debug)]
pub struct ButtonGroup {
    pub focus: FocusFlag,
    area: Rect,

    pub titles: Vec<String>,
    pub selected_index: usize,
    pub sender: Sender<ButtonGroupEvent>,
}

#[derive(Debug)]
pub enum ButtonGroupEvent {
    Selected(usize),
    Hovered(usize),
}

impl ButtonGroup {
    pub fn new(
        label: &str,
        titles: Vec<String>,
        selected_index: usize,
        sender: Sender<ButtonGroupEvent>,
    ) -> Self {
        Self {
            focus: FocusFlag::new().with_name(&format!("ToggleButton:{label}")),
            area: Rect::default(),
            titles,
            selected_index,
            sender,
        }
    }
    pub fn reset(&mut self) {
        self.selected_index = 0;
    }

    fn prev(&mut self) {
        if self.selected_index == 0 {
            self.selected_index = self.titles.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }
    fn next(&mut self) {
        if self.selected_index == self.titles.len() - 1 {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }
    }

    fn column_to_index(&self, column: u16) -> usize {
        let width = self.area.width / self.titles.len() as u16;
        let index = (column - self.area.x) / width;
        if index < self.titles.len() as u16 {
            index as usize
        } else {
            self.titles.len() - 1
        }
    }
}

impl Component for ButtonGroup {
    fn handle_action(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Left => {
                self.prev();
                ActionResult::Consumed
            }
            Action::Right => {
                self.next();
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        if let MouseEventKind::Moved = mouse.kind {
            self.selected_index = self.column_to_index(mouse.column);
            self.sender
                .send(ButtonGroupEvent::Hovered(self.selected_index))?;
            return Ok(None);
        }
        if let MouseEventKind::Up(_) = mouse.kind {
            self.selected_index = self.column_to_index(mouse.column);
            self.sender
                .send(ButtonGroupEvent::Selected(self.selected_index))?;
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        let width = self.area.width / self.titles.len() as u16;
        let button_layout =
            Layout::horizontal([Constraint::Ratio(1, self.titles.len() as u32)]).split(area);

        for title_index in 0..self.titles.len() {
            let lb = button_layout[0].offset(Offset {
                x: (title_index * width as usize) as i32,
                y: 0,
            });
            let is_selected = title_index == self.selected_index;
            frame.render_widget(themed_button(&self.titles[title_index], is_selected), lb);
        }
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}

impl HasFocus for ButtonGroup {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}
