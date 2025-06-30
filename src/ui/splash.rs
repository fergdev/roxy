use ratatui::prelude::*;
use tui_big_text::{BigText, PixelSize};

pub fn render_splash(frame: &mut Frame) {
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
        .lines(vec!["127.0.0.1:6969".into()])
        .build();

    frame.render_widget(name, chunks[0]);
    frame.render_widget(desc, chunks[1]);
    frame.render_widget(addr, chunks[2]);
}
