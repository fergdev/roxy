use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Constraint,
    prelude::Rect,
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, TableState, Wrap},
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
    focus: FocusFlag,
    table_state: TableState,
    area: Rect,
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
            focus: FocusFlag::new().with_name("FlowWsDetails"),
            table_state: TableState::default(),
            area: Rect::default(),
        }
    }
}

impl HasFocus for FlowDetailsWs {
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

impl Component for FlowDetailsWs {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        let data = self.state.borrow_and_update().data.clone();

        if data.is_empty() {
            let empty_text = vec![Line::raw("No messages")];
            let block = themed_block(Some("Messages"), self.focus.get());
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        } else {
            let rows: Vec<Row> = data
                .iter()
                .map(|field| Row::new(vec![Cell::from(Span::raw(field))]))
                .collect();

            let widths = [Constraint::Percentage(100)];

            frame.render_stateful_widget(
                themed_table(rows, widths, None, true),
                area,
                &mut self.table_state,
            );
        }

        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }
}
