use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

pub struct UiLogLayer {
    logs: Arc<Mutex<VecDeque<String>>>,
}

impl UiLogLayer {
    pub fn new(logs: Arc<Mutex<VecDeque<String>>>) -> Self {
        Self { logs }
    }
}

struct LogVisitor {
    message: Option<String>,
}

impl Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}

impl<S> Layer<S> for UiLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = LogVisitor { message: None };
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            if let Ok(mut logs) = self.logs.lock() {
                if logs.len() > 1000 {
                    logs.pop_front();
                }
                logs.push_back(message);
            }
        }
    }
}

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_log_popup(
    frame: &mut Frame,
    area: Rect,
    logs: &Arc<Mutex<VecDeque<String>>>,
    scroll_offset: usize,
) {
    let popup_area = centered_rect(80, 60, area);
    let paragraph = Paragraph::new(Text::from(
        logs.lock()
            .unwrap()
            .iter()
            .cloned()
            .map(Line::raw)
            .collect::<Vec<_>>(),
    ))
    .scroll((scroll_offset as u16, 0)) // <-- here
    .alignment(Alignment::Left)
    .block(Block::default().title("Log Output").borders(Borders::ALL));

    frame.render_widget(Clear, popup_area); // clears under the popup
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let center = popup_layout[1];
    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(center)[1]
}
