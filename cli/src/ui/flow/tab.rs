use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::layout::Rect;

pub struct TabComponent {
    pub focus: FocusFlag,
}

impl TabComponent {
    pub fn new(title: &str) -> Self {
        Self {
            focus: FocusFlag::new().with_name(title),
        }
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
        Rect::default()
    }
}

pub struct LineComponent {
    pub focus: FocusFlag,
}

impl LineComponent {
    pub fn new(name: &str) -> Self {
        Self {
            focus: FocusFlag::new().with_name(name),
        }
    }
}

impl HasFocus for LineComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}
