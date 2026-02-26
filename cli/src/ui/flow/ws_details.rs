use ratatui::{
    layout::Constraint,
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Wrap},
};
use roxy_proxy::flow::WsMessage;
use tokio::sync::{
    mpsc,
    watch::{self},
};
use tracing::debug;

use crate::ui::framework::{
    component::Component,
    theme::{themed_block, themed_table},
};

pub struct FlowDetailsWs {
    state: watch::Receiver<UiState>,
    focus: rat_focus::FocusFlag,
    table_state: ratatui::widgets::TableState,
}

#[derive(Default, Clone)]
struct UiState {
    data: Vec<String>,
}

impl FlowDetailsWs {
    pub fn new(mut cert_rx: mpsc::Receiver<Vec<WsMessage>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        tokio::spawn({
            async move {
                while let Some(messages) = cert_rx.recv().await {
                    let messages: Vec<String> = messages
                        .into_iter()
                        .map(|msg| format!("{:?}: {}", msg.direction, msg.message))
                        .collect();

                    ui_tx.send(UiState { data: messages }).unwrap_or_else(|e| {
                        debug!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: rat_focus::FocusFlag::new().with_name("FlowWsDetails"),
            table_state: ratatui::widgets::TableState::default(),
        }
    }
}

impl rat_focus::HasFocus for FlowDetailsWs {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::prelude::Rect {
        ratatui::prelude::Rect::default()
    }
}

impl Component for FlowDetailsWs {
    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let data = self.state.borrow_and_update().data.clone();

        if data.is_empty() {
            let empty_text = vec![Line::raw("No messages")];
            let block = themed_block(Some("Messages"), self.focus.get());
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        } else {
            let rows: Vec<Row> = data
                .iter()
                .map(|field| Row::new(vec![Cell::from(Span::raw(field))]))
                .collect();

            let widths = [Constraint::Percentage(100)];

            f.render_stateful_widget(
                themed_table(rows, widths, None, true),
                area,
                &mut self.table_state,
            );
        }

        Ok(())
    }
}
