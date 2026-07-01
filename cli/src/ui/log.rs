use crossterm::event::MouseEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{
    action::Action,
    ui::framework::{
        component::{DispatchError, DispatchResult},
        scroll::TwoAxisScrollState,
    },
};

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
    component::Component,
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
    area: Rect,
    scroll: TwoAxisScrollState,
}

impl HasFocus for LogViewer {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        self.area
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
            area: Rect::default(),
            scroll: TwoAxisScrollState::default(),
        }
    }
}

impl Component for LogViewer {
    fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> DispatchResult {
        self.scroll.handle_mouse_event(mouse);
        DispatchError::action(Action::FocusReq(self.focus.id()))
    }

    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        if self.scroll.handle_action(action) {
            DispatchError::stop()
        } else {
            Ok(())
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(80, 60, area);

        frame.render_widget(Clear, popup_area);

        let colors = with_theme(|theme| theme.colors.clone());
        if let Ok(logs) = self.logs.lock() {
            let mut width = 0;
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

                        width = width
                            .max(log_line.message.as_ref().map(|m| m.len()).unwrap_or(0) as u16);

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
            .scroll(self.scroll.offset())
            .alignment(Alignment::Left)
            .block(themed_block(Some("Logs"), true));

            self.scroll.set_content_height(logs.len() as u16);
            self.scroll.set_content_width(width);

            frame.render_widget(paragraph, popup_area);
            self.scroll.render(frame, popup_area);
        } else {
            frame.render_widget(Paragraph::new("No logs"), popup_area);
        }
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
