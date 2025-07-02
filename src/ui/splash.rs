use color_eyre::Result;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Widget},
};
use ratatui_image::{ResizeEncodeRender, protocol::StatefulProtocol};
use tui_big_text::{BigText, PixelSize};

use super::{component::Component, theme::with_theme};

pub struct Splash {
    port: u16,
}

impl Splash {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Component for Splash {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let colors = with_theme(|t| t.colors.clone());
        let bg = Block::default().style(Style::default().bg(colors.surface));
        frame.render_widget(bg, area);

        // let chunks = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints([
        //         Constraint::Percentage(33),
        //         Constraint::Percentage(33),
        //         Constraint::Percentage(34),
        //     ])
        //     .split(area);
        //
        // let text_style = Style::default().fg(colors.primary);
        //
        // let name = BigText::builder()
        //     .pixel_size(PixelSize::Full)
        //     // .alignment(Alignment::Center)
        //     .lines(vec!["Roxy".into()])
        //     .style(text_style)
        //     .build();
        //
        // let desc = BigText::builder()
        //     .pixel_size(PixelSize::Sextant)
        //     // .alignment(Alignment::Center)
        //     .lines(vec!["Rust MITM proxy".into()])
        //     .style(text_style)
        //     .build();
        //
        // let addr = BigText::builder()
        //     .pixel_size(PixelSize::Sextant)
        //     // .alignment(Alignment::Center)
        //     .lines(vec![format!("127.0.0.1:{}", self.port).into()])
        //     .style(text_style)
        //     .build();
        //
        // frame.render_widget(name, chunks[0]);
        // frame.render_widget(desc, chunks[1]);
        // frame.render_widget(addr, chunks[2]);
        Ok(())
    }
}

pub struct ImageWidget<'a> {
    pub protocol: &'a mut StatefulProtocol,
}

impl<'a> Widget for ImageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let a = ratatui::layout::Rect::new(area.x, area.y, area.width, area.height);

        self.protocol.render(a, buf);
    }
}

// fn render_image(buf: &mut Buffer, area: Rect, protocol: &mut StatefulProtocol) {
//     if let Ok(enc) = protocol.encode(area) {
//         for (y, line) in enc.lines.iter().enumerate() {
//             let y = area.y + y as u16;
//             if y >= area.y + area.height {
//                 break;
//             }
//
//             for (x, sym) in line.chars().enumerate() {
//                 let x = area.x + x as u16;
//                 if x >= area.x + area.width {
//                     break;
//                 }
//
//                 buf.get_mut(x, y).set_symbol(sym);
//                 // Optionally: set fg/bg color from protocol
//             }
//         }
//     }
// }
