use color_eyre::Result;
use rat_focus::HasFocus;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::event::Action;

use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Text},
    widgets::{Clear, Paragraph, Wrap},
};

use super::framework::{
    component::{ActionResult, Component},
    theme::{themed_block, with_theme},
    util::centered_rect,
};

pub struct UiLogLayer {
    logs: Arc<Mutex<VecDeque<LogLine>>>,
}

impl UiLogLayer {
    pub fn new(logs: Arc<Mutex<VecDeque<LogLine>>>) -> Self {
        Self { logs }
    }
}

pub struct LogLine {
    level: tracing::Level,
    message: Option<String>,
}

impl Visit for LogLine {
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
        let level = *event.metadata().level();
        let mut line = LogLine {
            level,
            message: None,
        };

        event.record(&mut line);

        if line.message.is_some() {
            if let Ok(mut logs) = self.logs.lock() {
                if logs.len() > 1000 {
                    logs.pop_front();
                }
                logs.push_back(line);
            }
        }
    }
}

pub struct LogViewer {
    focus: rat_focus::FocusFlag,
    logs: Arc<Mutex<VecDeque<LogLine>>>,
    v_scroll_offset: usize,
    h_scroll_offset: usize,
}

impl HasFocus for LogViewer {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl LogViewer {
    pub fn new(logs: Arc<Mutex<VecDeque<LogLine>>>) -> Self {
        Self {
            focus: rat_focus::FocusFlag::named("LogViewer"),
            logs,
            v_scroll_offset: 0,
            h_scroll_offset: 0,
        }
    }
}

impl Component for LogViewer {
    fn update(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Top => {
                self.v_scroll_offset = 0;
                ActionResult::Consumed
            }
            Action::Bottom => {
                self.v_scroll_offset = self.logs.lock().unwrap().len();
                ActionResult::Consumed
            }
            Action::Up => {
                if self.v_scroll_offset > 0 {
                    self.v_scroll_offset -= 1;
                }
                ActionResult::Consumed
            }
            Action::Down => {
                let logs_len = self.logs.lock().unwrap().len();
                if self.v_scroll_offset < logs_len.saturating_sub(1) {
                    self.v_scroll_offset += 1;
                }
                ActionResult::Consumed
            }
            Action::Right => {
                self.h_scroll_offset += 1;
                ActionResult::Consumed
            }
            Action::Left => {
                if self.h_scroll_offset > 0 {
                    self.h_scroll_offset -= 1;
                }
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 60, area);

        frame.render_widget(Clear, popup_area);

        let colors = with_theme(|t| t.colors.clone());
        let paragraph = Paragraph::new(Text::from(
            self.logs
                .lock()
                .unwrap()
                .iter()
                .map(|l| {
                    let style = match l.level {
                        tracing::Level::ERROR => ratatui::style::Style::default().fg(colors.error),
                        tracing::Level::WARN => ratatui::style::Style::default().fg(colors.warn),
                        tracing::Level::INFO => ratatui::style::Style::default().fg(colors.info),
                        tracing::Level::DEBUG => ratatui::style::Style::default().fg(colors.debug),
                        tracing::Level::TRACE => ratatui::style::Style::default().fg(colors.trace),
                    };
                    Line::from(l.message.clone().unwrap_or("this is bad".to_string())).style(style)
                })
                .collect::<Vec<_>>(),
        ))
        .wrap(Wrap { trim: false })
        .scroll((self.v_scroll_offset as u16, self.h_scroll_offset as u16))
        .alignment(Alignment::Left)
        .block(themed_block(Some("Logs"), true));

        frame.render_widget(paragraph, popup_area);
        Ok(())
    }
}
