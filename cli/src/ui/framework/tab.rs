use color_eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect, text::Line};

use crate::{
    action::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_tabs,
    },
};

#[derive(Debug)]
pub struct TabComponent {
    pub focus: FocusFlag,
    pub area: Rect,

    pub title: String,
    pub current_tab: usize,
    pub tabs: Vec<String>,
}

impl TabComponent {
    pub fn new(title: String, tabs: Vec<String>) -> Self {
        Self {
            focus: FocusFlag::new().with_name(&title),
            current_tab: 0,
            title,
            tabs,
            area: Rect::default(),
        }
    }

    pub fn next(&mut self) {
        if self.current_tab + 1 < self.tabs.len() {
            self.current_tab += 1;
        } else {
            self.start();
        }
    }

    pub fn prev(&mut self) {
        if self.current_tab == 0 {
            self.end();
        } else {
            self.current_tab -= 1;
        }
    }
    pub fn start(&mut self) {
        self.current_tab = 0;
    }
    pub fn end(&mut self) {
        self.current_tab = self.tabs.len() - 1;
    }
}
impl Component for TabComponent {
    fn area(&self) -> Rect {
        self.area
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;

        let tab_titles: Vec<Line> = self.tabs.iter().map(Line::raw).collect();
        let tab_index = self.current_tab;
        let tabs = themed_tabs(Some(&self.title), tab_titles, tab_index, self.focus.get());
        frame.render_widget(tabs, area);
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
        let mut res = ActionResult::Consumed;
        match action {
            Action::Left => {
                self.prev();
            }
            Action::Right => {
                self.next();
            }
            Action::Start => {
                self.start();
            }
            Action::End => {
                self.end();
            }
            _ => {
                res = ActionResult::Ignored;
            }
        }
        res
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<Option<Action>> {
        match mouse_event.kind {
            MouseEventKind::Up(_) => {
                let component_up_x = mouse_event.column - self.area.x;
                let mut tab_width = 0;
                for (index, title) in self.tabs.iter().enumerate() {
                    tab_width += title.len() as u16 + 3;
                    if tab_width > component_up_x {
                        self.current_tab = index;
                        return Ok(None);
                    }
                }
                self.end();
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

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}

impl HasFocus for TabComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}
