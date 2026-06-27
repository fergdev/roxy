use color_eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::action::Action;

use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
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
    level: Level,
    message: Option<String>,
}

impl Visit for LogLine {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
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

        if line.message.is_some()
            && let Ok(mut logs) = self.logs.lock()
        {
            if logs.len() > 1000 {
                logs.pop_front();
            }
            logs.push_back(line);
        }
    }
}

pub struct LogViewer {
    focus: FocusFlag,
    logs: Arc<Mutex<VecDeque<LogLine>>>,
    v_scroll_offset: usize,
    h_scroll_offset: usize,
}

impl HasFocus for LogViewer {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl LogViewer {
    pub fn new(logs: Arc<Mutex<VecDeque<LogLine>>>) -> Self {
        Self {
            focus: FocusFlag::new().with_name("LogViewer"),
            logs,
            v_scroll_offset: 0,
            h_scroll_offset: 0,
        }
    }
}

impl Component for LogViewer {
    fn dispatch_action(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Top => {
                self.v_scroll_offset = 0;
                ActionResult::Consumed
            }
            Action::Bottom => {
                if let Ok(guard) = self.logs.lock() {
                    self.v_scroll_offset = guard.len();
                }

                ActionResult::Consumed
            }
            Action::Up => {
                if self.v_scroll_offset > 0 {
                    self.v_scroll_offset -= 1;
                }
                ActionResult::Consumed
            }
            Action::Down => {
                if let Ok(guard) = self.logs.lock()
                    && self.v_scroll_offset < guard.len().saturating_sub(1)
                {
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

        let colors = with_theme(|theme| theme.colors.clone());
        if let Ok(logs) = self.logs.lock() {
            let paragraph = Paragraph::new(Text::from(
                logs.iter()
                    .map(|log_line| {
                        let color = match log_line.level {
                            Level::ERROR => colors.error,
                            Level::WARN => colors.warn,
                            Level::INFO => colors.info,
                            Level::DEBUG => colors.debug,
                            Level::TRACE => colors.trace,
                        };

                        Line::from(
                            log_line
                                .message
                                .clone()
                                .unwrap_or("this is bad".to_string()),
                        )
                        .style(Style::default().fg(color))
                    })
                    .collect::<Vec<_>>(),
            ))
            .wrap(Wrap { trim: false })
            .scroll((self.v_scroll_offset as u16, self.h_scroll_offset as u16))
            .alignment(Alignment::Left)
            .block(themed_block(Some("Logs"), true));
            frame.render_widget(paragraph, popup_area);
        } else {
            frame.render_widget(Paragraph::new("No logs"), popup_area);
        }

        Ok(())
    }
}
