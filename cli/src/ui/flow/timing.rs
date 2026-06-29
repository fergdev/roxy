use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect};
use roxy_proxy::flow::Timing;
use time::OffsetDateTime;
use tokio::sync::{mpsc, watch};

use crate::ui::framework::{component::Component, paragraph::kv_paragraph};

struct State {
    lines: Vec<(String, String)>,
}

pub struct FlowTiming {
    state: watch::Receiver<State>,
    focus: FocusFlag,
    area: Rect,
}

impl FlowTiming {
    pub fn new(mut rx: mpsc::Receiver<Timing>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(State { lines: vec![] });

        tokio::spawn({
            async move {
                while let Some(timing) = rx.recv().await {
                    let lines = vec![
                        timing_line("client_conn_established", &timing.client_conn_established),
                        timing_line("server_conn_initiated", &timing.server_conn_initiated),
                        timing_line(
                            "server_conn_http_handshake",
                            &timing.server_conn_http_handshake,
                        ),
                        timing_line(
                            "server_conn_TCP_handshake",
                            &timing.server_conn_tcp_handshake,
                        ),
                        timing_line(
                            "server_conn_TLS_handshake",
                            &timing.server_conn_tls_handshake,
                        ),
                        timing_line(
                            "client_conn_TLS_handshake",
                            &timing.client_conn_tls_handshake,
                        ),
                        timing_line("first_reques_byte", &timing.first_request_bytes),
                        timing_line("request_complete", &timing.request_complete),
                        timing_line("first_respons_byte", &timing.first_response_bytes),
                        timing_line("response_complete", &timing.response_complete),
                        timing_line("client_conn_closed", &timing.client_conn_closed),
                        timing_line("server_conn_closed", &timing.server_conn_closed),
                    ];
                    ui_tx.send(State { lines }).unwrap_or_else(|e| {
                        tracing::debug!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: FocusFlag::new().with_name("FlowTiming"),
            area: Rect::default(),
        }
    }
}

fn timing_line(key: &str, time: &Option<OffsetDateTime>) -> (String, String) {
    (
        key.to_owned(),
        time.map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
    )
}

impl HasFocus for FlowTiming {
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

impl Component for FlowTiming {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        let state = self.state.borrow();
        kv_paragraph(
            &state.lines,
            frame,
            area,
            Some("Timing"),
            self.focus.get(),
            (0, 0),
        );
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
