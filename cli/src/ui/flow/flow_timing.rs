use chrono::{DateTime, Utc};
use rat_focus::HasFocus;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};
use roxy_proxy::flow::Timing;
use tokio::sync::{mpsc, watch};

use crate::ui::framework::{component::Component, theme::themed_block};

struct State {
    lines: Vec<String>,
}

pub struct FlowTiming {
    state: watch::Receiver<State>,
    focus: rat_focus::FocusFlag,
}

impl FlowTiming {
    pub fn new(mut rx: mpsc::Receiver<Timing>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(State { lines: vec![] });

        tokio::spawn({
            async move {
                while let Some(timing) = rx.recv().await {
                    let lines = vec![
                        timing_line(&timing.client_conn_established, "client_conn_established"),
                        timing_line(&timing.server_conn_initiated, "server_conn_initiated"),
                        timing_line(
                            &timing.server_conn_tcp_handshake,
                            "server_conn_TCP_handshake",
                        ),
                        timing_line(
                            &timing.server_conn_tls_handshake,
                            "server_conn_TLS_handshake",
                        ),
                        timing_line(
                            &timing.client_conn_tls_handshake,
                            "client_conn_TLS_handshake",
                        ),
                        timing_line(&timing.first_reques_byte, "first_reques_byte"),
                        timing_line(&timing.request_complet_, "request_complet_"),
                        timing_line(&timing.first_respons_byte, "first_respons_byte"),
                        timing_line(&timing.response_complet_, "response_complet_"),
                        timing_line(&timing.client_conn_closed, "client_conn_closed"),
                        timing_line(&timing.server_conn_closed, "server_conn_closed"),
                    ];
                    ui_tx.send(State { lines }).unwrap_or_else(|e| {
                        tracing::debug!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: rat_focus::FocusFlag::named("FlowTiming"),
        }
    }
}

fn timing_line(time: &Option<DateTime<Utc>>, key: &str) -> String {
    format!(
        "{}: {}",
        key,
        time.map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    )
}

impl HasFocus for FlowTiming {
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

impl Component for FlowTiming {
    fn render(&mut self, f: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        f.render_widget(
            Paragraph::new(self.state.borrow().lines.join("\n"))
                .block(themed_block(Some("Timing"), self.focus.get())),
            area,
        );
        Ok(())
    }
}
