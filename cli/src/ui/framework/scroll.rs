use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::{Frame, layout::Rect, widgets::ScrollbarState};

use crate::{
    action::Action,
    ui::framework::scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
};

const PAGE_SCROLL_AMOUNT: usize = 10;

#[derive(Debug, Default)]
pub struct TwoAxisScrollState {
    vertical: ScrollbarState,
    horizontal: ScrollbarState,
}

impl TwoAxisScrollState {
    pub fn handle_mouse_event(&mut self, mouse: &MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollLeft => {
                self.horizontal.prev();
            }
            MouseEventKind::ScrollRight => {
                self.horizontal.next();
            }
            MouseEventKind::ScrollUp => {
                self.vertical.prev();
            }
            MouseEventKind::ScrollDown => {
                self.vertical.next();
            }
            _ => {}
        }
    }

    pub fn handle_action(&mut self, action: &Action) -> bool {
        let mut consumed = true;
        match action {
            Action::Top => {
                self.vertical.first();
            }
            Action::Bottom => {
                self.vertical.last();
            }
            Action::Start => {
                self.horizontal.first();
            }
            Action::End => {
                self.horizontal.last();
            }
            Action::PageUp => {
                let curr_pos = self.vertical.get_position();
                self.vertical = self
                    .vertical
                    .position(curr_pos.saturating_sub(PAGE_SCROLL_AMOUNT));
            }
            Action::PageDown => {
                let curr_pos = self.vertical.get_position();
                self.vertical = self
                    .vertical
                    .position(curr_pos.saturating_add(PAGE_SCROLL_AMOUNT));
            }
            Action::PageLeft => {
                let curr_pos = self.horizontal.get_position();
                self.horizontal = self
                    .horizontal
                    .position(curr_pos.saturating_sub(PAGE_SCROLL_AMOUNT));
            }
            Action::PageRight => {
                let curr_pos = self.horizontal.get_position();
                self.horizontal = self
                    .horizontal
                    .position(curr_pos.saturating_add(PAGE_SCROLL_AMOUNT));
            }
            Action::Up => {
                self.vertical.prev();
            }
            Action::Down => {
                self.vertical.next();
            }
            Action::Left => {
                self.horizontal.prev();
            }
            Action::Right => {
                self.horizontal.next();
            }
            _ => consumed = false,
        }
        consumed
    }

    /// Offest to be passed to ratatui widgets
    pub(crate) fn offset(&self) -> (u16, u16) {
        (
            self.vertical.get_position() as u16,
            self.horizontal.get_position() as u16,
        )
    }
    pub(crate) fn set(&mut self, content_size: (u16, u16), viewport_size: (u16, u16)) {
        self.set_content_width(content_size.0);
        self.set_content_height(content_size.1);
        self.horizontal = self
            .horizontal
            .viewport_content_length(viewport_size.0 as usize);
        self.vertical = self
            .vertical
            .viewport_content_length(viewport_size.1 as usize);
    }

    fn set_content_height(&mut self, len: u16) {
        self.vertical = self.vertical.content_length(len as usize)
    }
    fn set_content_width(&mut self, len: u16) {
        self.horizontal = self.horizontal.content_length(len as usize)
    }

    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_vertical_scrollbar(frame, area, &mut self.vertical);
        render_horizontal_scrollbar(frame, area, &mut self.horizontal);
    }
}
