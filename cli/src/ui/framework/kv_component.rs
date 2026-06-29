use color_eyre::eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph, ScrollbarState},
};

use crate::{
    action::Action,
    ui::framework::{
        component::Component,
        paragraph::kv_paragraph,
        scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
    },
};

use super::component::ActionResult;

pub(crate) struct KvComponent {
    pub focus: FocusFlag,
    pub area: Rect,

    pub state: Option<Vec<(String, String)>>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}
impl KvComponent {
    pub(crate) fn new(title: &str) -> Self {
        Self {
            focus: FocusFlag::new().with_name(title),
            area: Rect::default(),
            state: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }
    pub fn set_state(&mut self, data: Vec<(String, String)>) {
        self.scroll_index_vertical = ScrollbarState::new(data.len());
        let width = data.iter().fold(0, |acc, (k, v)| {
            let width = k.len() + v.len() + 5; // 5 for padding and separator
            width.max(acc)
        });
        self.scroll_index_horizontal = ScrollbarState::new(width);
        self.state = Some(data);
    }
}

impl Component for KvComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        if let Some(data) = &self.state {
            kv_paragraph(
                data,
                frame,
                area,
                Some(self.focus.name().as_ref()),
                self.focus.get(),
                (
                    self.scroll_index_vertical.get_position() as u16,
                    self.scroll_index_horizontal.get_position() as u16,
                ),
            );
            render_vertical_scrollbar(frame, area, &mut self.scroll_index_vertical);
            render_horizontal_scrollbar(frame, area, &mut self.scroll_index_horizontal);
        } else {
            frame.render_widget(
                Paragraph::new("No data").block(Block::default().title("Hello")),
                area,
            );
        }
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
        let mut res = ActionResult::Consumed;
        match action {
            Action::Top => {
                self.scroll_index_vertical.first();
            }
            Action::Bottom => {
                self.scroll_index_vertical.last();
            }
            Action::Start => {
                self.scroll_index_horizontal.first();
            }
            Action::End => {
                self.scroll_index_horizontal.last();
            }
            Action::Up => {
                self.scroll_index_vertical.prev();
            }
            Action::Down => {
                self.scroll_index_vertical.next();
            }
            Action::Left => {
                self.scroll_index_horizontal.prev();
            }
            Action::Right => {
                self.scroll_index_horizontal.next();
            }
            _ => res = ActionResult::Ignored,
        }
        res
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<Option<Action>> {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_index_vertical.prev();
            }
            MouseEventKind::ScrollDown => {
                self.scroll_index_vertical.next();
            }
            MouseEventKind::ScrollLeft => {
                self.scroll_index_horizontal.prev();
            }
            MouseEventKind::ScrollRight => {
                self.scroll_index_horizontal.next();
            }
            MouseEventKind::Moved => {
                if !self.focus.get() {
                    return Ok(Some(Action::FocusReq(self.focus.widget_id())));
                }
            }
            _ => {}
        }

        Ok(None)
    }
}

impl HasFocus for KvComponent {
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
