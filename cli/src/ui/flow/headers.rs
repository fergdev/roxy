use color_eyre::eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use hyper::HeaderMap;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    text::Span,
    widgets::{Cell, Clear, Row, TableState},
};
use tokio::sync::{
    mpsc::{self},
    watch,
};
use tracing::error;

use crate::{
    action::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::{themed_block, themed_table},
    },
};

pub struct FlowDetailsHeaders {
    headers: watch::Receiver<Option<HeaderMap>>,
    focus: FocusFlag,
    area: Rect,
    table_state: TableState,
}

impl FlowDetailsHeaders {
    pub fn new(mut req_rx: mpsc::Receiver<HeaderMap>) -> Self {
        let (headers_tx, headers_rx) = watch::channel(None);

        tokio::spawn(async move {
            while let Some(req) = req_rx.recv().await {
                headers_tx.send(Some(req)).unwrap_or_else(|e| {
                    error!("Failed to send headers: {}", e);
                });
            }
        });

        Self {
            headers: headers_rx,
            focus: FocusFlag::new().with_name("FlowHeaders"),
            area: Rect::default(),
            table_state: TableState::default(),
        }
    }
}

impl HasFocus for FlowDetailsHeaders {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl Component for FlowDetailsHeaders {
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        if self.focus.get() {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.table_state.scroll_up_by(1);
                    return Ok(None);
                }
                MouseEventKind::ScrollDown => {
                    self.table_state.scroll_down_by(1);
                    return Ok(None);
                }
                _ => {}
            }
        } else {
            return Ok(Some(Action::FocusReq(self.focus.id())));
        }
        Ok(None)
    }
    fn handle_action(&mut self, action: Action) -> ActionResult {
        if self.focus.get() {
            let mut res = ActionResult::Consumed;
            match action {
                Action::Up => {
                    self.table_state.select_previous();
                }
                Action::Down => {
                    self.table_state.select_next();
                }
                Action::PageUp => {
                    self.table_state.scroll_up_by(self.area.height);
                }
                Action::PageDown => {
                    self.table_state.scroll_down_by(self.area.height);
                }
                Action::Top => {
                    self.table_state.select(Some(0));
                }
                Action::Bottom => {
                    let headers = self.headers.borrow_and_update();
                    let headers_size = headers.as_ref().map(|h| h.len()).unwrap_or(0);
                    self.table_state.scroll_down_by(headers_size as u16);
                }
                _ => res = ActionResult::Ignored,
            }
            res
        } else {
            ActionResult::Ignored
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = area;
        frame.render_widget(Clear, area);
        let headers = self.headers.borrow_and_update();
        match headers.as_ref() {
            Some(headers) => {
                let header_style = Style::default();
                let mut rows = vec![];
                for (header_key, header_value) in headers {
                    let header_value_str = header_value.to_str().unwrap_or("error").to_string();

                    rows.push(
                        Row::new(vec![
                            Cell::from(Span::styled(header_key.to_string(), header_style)),
                            Cell::from(header_value_str),
                        ])
                        .height(1_u16),
                    );
                }
                let widths = [Constraint::Length(20), Constraint::Min(10)];
                let table = themed_table(rows, widths, Some("Headers"), self.focus.get());

                frame.render_stateful_widget(table, area, &mut self.table_state);
            }
            None => {
                let paragraph = ratatui::widgets::Paragraph::new("No headers available")
                    .block(themed_block(Some("headers"), self.focus.get()));
                frame.render_widget(paragraph, area);
            }
        }

        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
