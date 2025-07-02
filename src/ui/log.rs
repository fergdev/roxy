use color_eyre::Result;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::event::Action;

use super::{component::Component, theme::themed_block, util::centered_rect};
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
    widgets::Paragraph,
};

pub struct LogViewer {
    logs: Arc<Mutex<VecDeque<String>>>,
    v_scroll_offset: usize,
    h_scroll_offset: usize,
}

impl LogViewer {
    pub fn new(logs: Arc<Mutex<VecDeque<String>>>) -> Self {
        Self {
            logs,
            v_scroll_offset: 0,
            h_scroll_offset: 0,
        }
    }
}

impl Component for LogViewer {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Top => {
                self.v_scroll_offset = 0;
            }
            Action::Bottom => {
                self.v_scroll_offset = self.logs.lock().unwrap().len();
            }
            Action::Up => {
                if self.v_scroll_offset > 0 {
                    self.v_scroll_offset -= 1;
                }
            }
            Action::Down => {
                let logs_len = self.logs.lock().unwrap().len();
                if self.v_scroll_offset < logs_len.saturating_sub(1) {
                    self.v_scroll_offset += 1;
                }
            }
            Action::Right => {
                self.h_scroll_offset += 1;
            }
            Action::Left => {
                if self.h_scroll_offset > 0 {
                    self.h_scroll_offset -= 1;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);

        let paragraph = Paragraph::new(Text::from(
            self.logs
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .map(Line::raw)
                .collect::<Vec<_>>(),
        ))
        .scroll((self.v_scroll_offset as u16, self.h_scroll_offset as u16))
        .alignment(Alignment::Left)
        .block(themed_block("Logs"));

        frame.render_widget(paragraph, popup_area);
        Ok(())
    }
}
