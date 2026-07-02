use hyper::Method;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Cell, Row, ScrollbarState, TableState},
};
use roxy_proxy::flow_store::FlowStore;
use tokio::{sync::watch, task::JoinHandle};
use tracing::error;

use crate::{
    action::Action,
    ui::framework::{
        component::{Component, DispatchCancellation, DispatchResult},
        scrollbar::render_vertical_scrollbar,
        theme::{themed_table, with_theme},
    },
};

#[derive(Debug, Clone)]
struct UiFlow {
    id: i64,
    method: Method,
    uri: String,
    response: UiResponse,
    size: String,
    timing: String,
}

#[derive(Debug, Clone)]
enum UiResponse {
    Error,
    Loading,
    Ok(u16),
}

#[derive(Clone, Default)]
struct UiState {
    flows: Vec<UiFlow>,
}

pub struct FlowList {
    focus: FocusFlag,
    state: TableState,
    ui_rx: watch::Receiver<UiState>,
    listener_handle: JoinHandle<()>,
    area: Rect,
}

impl HasFocus for FlowList {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl FlowList {
    pub fn new(flow_store: FlowStore) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());
        let handle = FlowList::start_listener(flow_store.clone(), ui_tx);

        Self {
            focus: FocusFlag::new().with_name("FlowList"),
            state: TableState::default().with_selected(0),
            ui_rx,
            listener_handle: handle,
            area: Rect::default(),
        }
    }

    fn start_listener(flow_store: FlowStore, ui_tx: watch::Sender<UiState>) -> JoinHandle<()> {
        let flow_store = flow_store.clone();

        tokio::spawn(async move {
            let mut flow_rx = flow_store.subscribe();
            while flow_rx.changed().await.is_ok() {
                let ids = flow_store.ordered_ids.read().await;

                let mut flows = Vec::new();
                for id in ids.iter() {
                    if let Some(entry) = flow_store.flows.get(id) {
                        let flow = entry.value().read().await;

                        let response = if let Some(resp) = flow.response.as_ref() {
                            UiResponse::Ok(resp.status.as_u16())
                        } else if flow.error.as_ref().is_some() {
                            UiResponse::Error
                        } else {
                            UiResponse::Loading
                        };

                        let (method, line) = match flow.request.as_ref() {
                            Some(req) => (req.method.clone(), req.line_pretty()),
                            None => (Method::GET, "?????".to_string()),
                        };

                        let start_time = flow.timing.client_conn_established;
                        let end_time = flow.timing.server_conn_closed;
                        let timing = if let (Some(start), Some(end)) = (start_time, end_time) {
                            let diff = end - start;
                            format_duration(diff)
                        } else {
                            "N/A".to_string()
                        };

                        let size_request =
                            flow.request.as_ref().map(|r| r.body.len()).unwrap_or(0) as u64;
                        let size_response =
                            flow.response.as_ref().map(|r| r.body.len()).unwrap_or(0) as u64;
                        let total_size = size_request + size_response;
                        let size = format_bytes(total_size);

                        flows.push(UiFlow {
                            id: *id,
                            method,
                            uri: line,
                            response,
                            size,
                            timing,
                        });
                    }
                }
                if let Err(e) = ui_tx.send(UiState { flows }) {
                    error!("error posting ui state {e}");
                }
            }
        })
    }

    fn next_row(&mut self) {
        self.state.select_next();
    }

    fn previous_row(&mut self) {
        self.state.select_previous();
    }

    pub fn selected_id(&self) -> Option<i64> {
        if let Some(selected) = self.state.selected() {
            let state = self.ui_rx.borrow();
            if selected < state.flows.len() {
                Some(state.flows[selected].id)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Drop for FlowList {
    fn drop(&mut self) {
        self.listener_handle.abort();
    }
}

impl Component for FlowList {
    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        match action {
            Action::Down => {
                self.next_row();
                DispatchCancellation::stop()
            }
            Action::Up => {
                self.previous_row();
                DispatchCancellation::stop()
            }
            Action::PageUp => {
                let position = self.state.selected().unwrap_or(0);
                let height = self.area.height.saturating_sub(2);
                self.state
                    .select(Some(position.saturating_sub(height as usize)));
                DispatchCancellation::stop()
            }
            Action::PageDown => {
                let position = self.state.selected().unwrap_or(0);
                let height = self.area.height.saturating_sub(2);
                let max_position = self.ui_rx.borrow().flows.len() - 1;
                self.state.select(Some(
                    position.saturating_add(height as usize).min(max_position),
                ));
                DispatchCancellation::stop()
            }
            Action::Top => {
                self.state.select(Some(0));
                DispatchCancellation::stop()
            }
            Action::Bottom => {
                self.state.select(Some(self.ui_rx.borrow().flows.len()));
                DispatchCancellation::stop()
            }
            _ => Ok(()),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        let guard = self.ui_rx.borrow_and_update();

        let mut rows = vec![];
        for flow in &guard.flows {
            let status = with_theme(|theme| match &flow.response {
                UiResponse::Error => Span::styled(" ERR ", Style::default().fg(theme.colors.error)),
                UiResponse::Loading => {
                    Span::styled(" ... ", Style::default().fg(theme.colors.warn))
                }
                UiResponse::Ok(code) => Span::styled(
                    format!(" {code} "),
                    Style::default().fg(theme.colors.success),
                ),
            });
            let line = Line::from(vec![
                Span::styled(
                    flow.method.to_string(),
                    Style::default().fg(method_color(&flow.method)),
                ),
                Span::styled("   ", Style::default()),
                status,
                Span::styled(&flow.uri, Style::default().fg(Color::Cyan)),
                Span::styled("   ", Style::default()),
                Span::styled(&flow.size, Style::default().fg(Color::Cyan)),
                Span::styled(" ", Style::default()),
                Span::styled(&flow.timing, Style::default().fg(Color::Cyan)),
            ]);
            rows.push(Row::new(vec![Cell::new(line)]));
        }

        frame.render_stateful_widget(
            themed_table(rows, [Constraint::Fill(1)], Some("Flows"), self.focus.get()),
            area,
            &mut self.state,
        );
        let mut sb =
            ScrollbarState::new(guard.flows.len()).position(self.state.selected().unwrap_or(0));
        render_vertical_scrollbar(frame, area, &mut sb);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;
fn format_bytes(bytes: u64) -> String {
    match bytes {
        0..KB => format!("{bytes}b"),
        KB..MB => format!("{}kb", bytes / KB),
        MB..GB => format!("{}mb", bytes / MB),
        GB..TB => format!("{}gb", bytes / GB),
        _ => format!("{}tb", bytes / TB),
    }
}

pub fn format_duration(duration: time::Duration) -> String {
    let nanos = duration.whole_nanoseconds();

    if nanos < 1_000 {
        return format!("{nanos}ns");
    }

    if nanos < 1_000_000 {
        return format!("{}µs", nanos / 1_000);
    }

    if nanos < 1_000_000_000 {
        return format!("{}ms", nanos / 1_000_000);
    }

    let secs = duration.as_seconds_f64();
    if secs < 60.0 {
        return format!("{secs:.2}s");
    }

    if secs < 3600.0 {
        return format!("{}m {}s", secs / 60.0, secs % 60.0);
    }

    format!("{}h {}m", secs / 3600.0, (secs % 3600.0) / 60.0)
}

fn method_color(method: &Method) -> Color {
    match *method {
        Method::GET => Color::Green,
        Method::POST => Color::Green,
        Method::PUT => Color::Green,
        Method::DELETE => Color::Green,
        Method::HEAD => Color::Green,
        Method::OPTIONS => Color::Green,
        Method::CONNECT => Color::Green,
        Method::PATCH => Color::Green,
        Method::TRACE => Color::Green,
        _ => Color::Yellow,
    }
}
