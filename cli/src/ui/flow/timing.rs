use color_eyre::eyre::Result;
use crossterm::event::MouseEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect};
use roxy_proxy::flow::Timing;
use time::OffsetDateTime;
use tokio::sync::{mpsc, watch};

use crate::{
    action::Action,
    ui::framework::{
        component::{ActionResult, Component},
        paragraph::kv_paragraph,
        scroll::TwoAxisScrollState,
    },
};

#[derive(Default)]
struct State {
    width: usize,
    height: usize,
    lines: Vec<(String, String)>,
}

pub struct FlowTiming {
    state: watch::Receiver<State>,
    focus: FocusFlag,
    area: Rect,
    scroll: TwoAxisScrollState,
    state_handle: tokio::task::JoinHandle<()>,
}

impl FlowTiming {
    pub fn new(mut rx: mpsc::Receiver<Timing>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(State::default());

        let state_handle = tokio::spawn({
            async move {
                while let Some(timing) = rx.recv().await {
                    let lines = render_timing(&timing);
                    let height = lines.len();
                    let width = lines
                        .iter()
                        .map(|(k, v)| k.len() + v.len())
                        .max()
                        .unwrap_or(0);
                    ui_tx
                        .send(State {
                            lines: render_timing(&timing),
                            width,
                            height,
                        })
                        .unwrap_or_else(|e| {
                            tracing::debug!("Failed to send UI state update: {}", e);
                        });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: FocusFlag::new().with_name("FlowTiming"),
            area: Rect::default(),
            scroll: TwoAxisScrollState::default(),
            state_handle,
        }
    }
}
impl Drop for FlowTiming {
    fn drop(&mut self) {
        self.state_handle.abort();
    }
}
fn render_timing(timing: &Timing) -> Vec<(String, String)> {
    vec![
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
        timing_line("first_request_byte", &timing.first_request_bytes),
        timing_line("request_complete", &timing.request_complete),
        timing_line("first_response_byte", &timing.first_response_bytes),
        timing_line("response_complete", &timing.response_complete),
        timing_line("client_conn_closed", &timing.client_conn_closed),
        timing_line("server_conn_closed", &timing.server_conn_closed),
    ]
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
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        let state = self.state.borrow();
        self.scroll
            .set_content_size((state.width as u16, state.height as u16));
        kv_paragraph(
            &state.lines,
            frame,
            area,
            Some("Timing"),
            self.focus.get(),
            self.scroll.offset(),
        );

        self.scroll.render(frame, area);
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        self.scroll.handle_mouse_event(mouse);
        Ok(Some(Action::FocusReq(self.focus.id())))
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
        if self.scroll.handle_action(action.clone()) {
            ActionResult::Consumed
        } else {
            ActionResult::Ignored
        }
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
