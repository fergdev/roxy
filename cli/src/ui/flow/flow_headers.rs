use hyper::HeaderMap;
use ratatui::{
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
    event::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::{themed_block, themed_table},
    },
};

pub struct FlowDetailsHeaders {
    headers: watch::Receiver<Option<HeaderMap>>,
    focus: rat_focus::FocusFlag,
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
            focus: rat_focus::FocusFlag::new().with_name("FlowHeaders"),
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
                let header_style = Style::default().bold();
                let mut rows = vec![];
                for (k, v) in headers {
                    let value = v.to_str().unwrap_or("error").to_string();

                    rows.push(
                        Row::new(vec![
                            Cell::from(Span::styled(k.clone().to_string(), header_style)),
                            Cell::from(value),
                        ])
                        .height(1_u16),
                    );
                }
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
