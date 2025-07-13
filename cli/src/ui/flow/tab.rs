use rat_focus::HasFocus;
use ratatui::layout::Rect;

pub struct TabComponent {
    pub focus: rat_focus::FocusFlag,
}

impl TabComponent {
    pub fn new(title: &str) -> Self {
        Self {
            focus: rat_focus::FocusFlag::named(title),
        }
    }
}

impl HasFocus for TabComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

pub struct LineComponent {
    pub focus: rat_focus::FocusFlag,
}

impl LineComponent {
    pub fn new(name: &str) -> Self {
        Self {
            focus: rat_focus::FocusFlag::named(name),
        }
    }
}

impl HasFocus for LineComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}
