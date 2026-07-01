use crossterm::event::MouseEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph},
};

use crate::{
    action::Action,
    ui::framework::{
        component::{Component, DispatchError, DispatchResult},
        paragraph::kv_paragraph,
        scroll::TwoAxisScrollState,
    },
};

pub(crate) struct KvComponent {
    pub focus: FocusFlag,
    pub area: Rect,

    pub state: Option<Vec<(String, String)>>,

    scroll: TwoAxisScrollState,
}
impl KvComponent {
    pub(crate) fn new(title: &str) -> Self {
        Self {
            focus: FocusFlag::new().with_name(title),
            area: Rect::default(),
            state: None,
            scroll: TwoAxisScrollState::default(),
        }
    }
    pub fn set_state(&mut self, data: Vec<(String, String)>) {
        self.scroll.set_content_height(data.len() as u16);
        let width = data.iter().fold(0, |acc, (k, v)| {
            let width = k.len() + v.len() + 5; // 5 for padding and separator
            width.max(acc)
        });
        self.scroll.set_content_width(width as u16);
        self.state = Some(data);
    }
}

impl Component for KvComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        if let Some(data) = &self.state {
            kv_paragraph(
                data,
                frame,
                area,
                Some(self.focus.name().as_ref()),
                self.focus.get(),
                self.scroll.offset(),
            );
            self.scroll.render(frame, area);
        } else {
            frame.render_widget(
                Paragraph::new("No data").block(Block::default().title("Hello")),
                area,
            );
        }
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> DispatchResult {
        if !self.focus.get() {
            return DispatchError::action(Action::FocusReq(self.focus.id()));
        }

        self.scroll.handle_mouse_event(mouse);
        Ok(())
    }
    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        if self.scroll.handle_action(action) {
            DispatchError::stop()
        } else {
            Ok(())
        }
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
