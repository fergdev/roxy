use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Cell, Clear, Row, TableState},
};
use tokio::sync::{
    mpsc::{self},
    watch,
};
use tracing::error;

use crate::{
    event::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::{themed_block, themed_table},
    },
};

pub struct FlowDetailsHeaders {
    headers: watch::Receiver<Option<HashMap<String, String>>>,
    focus: rat_focus::FocusFlag,
    table_state: TableState,
}

impl FlowDetailsHeaders {
    pub fn new(mut req_rx: mpsc::Receiver<HashMap<String, String>>) -> Self {
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
            focus: rat_focus::FocusFlag::named("FlowHeaders"),
            table_state: TableState::default(),
        }
    }
}

impl rat_focus::HasFocus for FlowDetailsHeaders {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::prelude::Rect {
        Rect::default()
    }
}

impl Component for FlowDetailsHeaders {
    fn update(&mut self, action: Action) -> ActionResult {
        if self.focus.get() {
            match action {
                Action::Up => {
                    self.table_state.select_previous();
                    ActionResult::Consumed
                }
                Action::Down => {
                    self.table_state.select_next();
                    ActionResult::Consumed
                }
                _ => ActionResult::Ignored,
            }
        } else {
            ActionResult::Ignored
        }
    }

    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        f.render_widget(Clear, area);
        let headers = self.headers.borrow_and_update();
        match headers.as_ref() {
            Some(headers) => {
                let width = area.width.saturating_sub(20).max(10) as usize;

                let header_style = Style::default().bold();
                let rows = headers.iter().map(|(k, v)| {
                    let (height, cell) = wrap_text(width, v);
                    Row::new(vec![
                        Cell::from(Span::styled(k.clone(), header_style)),
                        Cell::from(cell),
                    ])
                    .height(height as u16)
                });
                let widths = [Constraint::Length(20), Constraint::Min(10)];
                let table = themed_table(rows, widths, Some("Headers"), self.focus.get());

                f.render_stateful_widget(table, area, &mut self.table_state);
            }
            None => {
                let paragraph = ratatui::widgets::Paragraph::new("No headers available")
                    .block(themed_block(Some("headers"), self.focus.get()));
                f.render_widget(paragraph, area);
            }
        }

        Ok(())
    }
}

fn wrap_text(width: usize, str: &str) -> (usize, Text) {
    let mut lines = Vec::new();
    let mut i = 0;
    while i < str.len() {
        let mut end = i + width;
        if end > str.len() {
            end = str.len();
        }
        lines.push(Line::from(&str[i..end]));
        i += width;
    }

    (lines.len(), Text::from(lines))
}
