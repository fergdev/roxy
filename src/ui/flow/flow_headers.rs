use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::{Cell, Clear, Row, TableState},
};
use tokio::sync::{
    mpsc::{self},
    watch,
};
use tracing::debug;

use crate::{
    event::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_table,
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
                    tracing::debug!("Failed to send headers: {}", e);
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
            debug!("FlowDetailsHeaders update: {:?}", action);
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
        let headers = self.headers.borrow_and_update();
        match headers.as_ref() {
            Some(headers) => {
                let header_style = Style::default().bold();
                let rows = headers.iter().map(|(k, v)| {
                    Row::new(vec![
                        Cell::from(Span::styled(k.clone(), header_style)),
                        Cell::from(v.clone()),
                    ])
                });
                let widths = [Constraint::Length(20), Constraint::Min(10)];
                let table = themed_table(rows, widths, Some("Headers"), self.focus.get());

                f.render_widget(Clear, area);
                f.render_stateful_widget(table, area, &mut self.table_state);
            }
            None => {
                // TODO: theme
                let paragraph = ratatui::widgets::Paragraph::new("No headers available").block(
                    ratatui::widgets::Block::default()
                        .title("Headers")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                f.render_widget(paragraph, area);
            }
        }

        Ok(())
    }
}
