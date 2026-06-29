use color_eyre::eyre;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect};

use crate::ui::framework::component::Component;

pub struct LineComponent {
    pub focus: FocusFlag,
    area: Rect,
}

impl LineComponent {
    pub fn new(name: &str) -> Self {
        Self {
            focus: FocusFlag::new().with_name(name),
            area: Rect::default(),
        }
    }
}

impl Component for LineComponent {
    fn render(&mut self, _frame: &mut Frame, area: Rect) -> eyre::Result<()> {
        self.area = area;
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
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
        self.area
    }
}
