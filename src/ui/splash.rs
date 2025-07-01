use color_eyre::Result;
use ratatui::prelude::*;
use tui_big_text::{BigText, PixelSize};

use super::component::Component;

pub struct Splash {
    port: u16,
}

impl Splash {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Component for Splash {
    fn render(&mut self, frame: &mut Frame, _area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(frame.area());

        let name = BigText::builder()
            .pixel_size(PixelSize::Full)
            .alignment(Alignment::Center)
            .lines(vec!["Roxy".into()])
            .build();

        let desc = BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .alignment(Alignment::Center)
            .lines(vec!["Rust MITM proxy".into()])
            .build();

        let addr = BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .alignment(Alignment::Center)
            .lines(vec![format!("127.0.0.1:{}", self.port).into()])
            .build();

        frame.render_widget(name, chunks[0]);
        frame.render_widget(desc, chunks[1]);
        frame.render_widget(addr, chunks[2]);
        Ok(())
    }
}
