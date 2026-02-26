use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    prelude::{Constraint, Direction, Frame, Layout, Rect, Style},
    widgets::Block,
};
use tui_big_text::{BigText, PixelSize};

use super::framework::{component::Component, theme::with_theme};

pub struct Splash {
    focus: FocusFlag,
    port: u16,
}

impl Splash {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            focus: FocusFlag::new().with_name("Splash"),
        }
    }
}

impl HasFocus for Splash {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl Component for Splash {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let colors = with_theme(|t| t.colors.clone());
        let bg = Block::default().style(Style::default().bg(colors.surface));
        frame.render_widget(bg, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        let text_style = Style::new().fg(colors.primary);

        let name = BigText::builder()
            .pixel_size(PixelSize::Full)
            .centered()
            .style(text_style)
            .lines(vec!["Roxy".into()])
            .build();

        let desc = BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .centered()
            .lines(vec!["Rust MITM proxy".into()])
            .style(text_style)
            .build();

        let addr = BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .centered()
            .lines(vec![format!("127.0.0.1:{}", self.port).into()])
            .style(text_style)
            .build();

        frame.render_widget(name, chunks[0]);
        frame.render_widget(desc, chunks[1]);
        frame.render_widget(addr, chunks[2]);
        Ok(())
    }
}
