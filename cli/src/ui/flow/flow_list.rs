use color_eyre::Result;
use hyper::{Method, header::CONTENT_TYPE};
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, TableState},
};
use roxy_proxy::flow::FlowStore;
use tokio::{sync::watch, task::JoinHandle};
use tracing::error;

use crate::{
    app::ITEM_HEIGHT,
    event::Action,
    themed_row,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_table,
    },
};

#[derive(Debug, Clone)]
struct UiFlow {
    id: i64,
    method: Method,
    uri: String,
    response: Option<UiResponse>,
}

#[derive(Debug, Clone)]
struct UiResponse {
    code: u16,
    content_type: String,
    duration: i64,
}

#[derive(Clone, Default)]
struct UiState {
    flows: Vec<UiFlow>,
}

pub struct FlowList {
    focus: FocusFlag,
    flow_store: FlowStore,
    state: TableState,
    scroll_state: ScrollbarState,
    ui_rx: watch::Receiver<UiState>,
    shutdown_tx: watch::Sender<()>,
    listener_handle: Option<JoinHandle<()>>,
}

impl HasFocus for FlowList {
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

impl FlowList {
    pub fn new(flow_store: FlowStore) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let mut instance = Self {
            focus: FocusFlag::named("FlowList"),
            flow_store,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            ui_rx,
            listener_handle: None,
            shutdown_tx,
        };

        let handle = instance.start_listener(ui_tx, shutdown_rx);
        instance.listener_handle = Some(handle);

        instance
    }

    fn start_listener(
        &self,
        ui_tx: watch::Sender<UiState>,
        mut shutdown_rx: watch::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        let flow_store = self.flow_store.clone();

        tokio::spawn(async move {
            let mut flow_rx = flow_store.subscribe();

            loop {
                tokio::select! {
                    _ = flow_rx.changed() => {
                        let ids = flow_store.ordered_ids.read().await;

                        let mut flows = Vec::new();
                        for id in ids.iter() {
                            if let Some(entry) = flow_store.flows.get(id) {

                                let flow = entry.value().read().await;

                                let response = flow.response.as_ref()
                                    .map(|r| UiResponse{
                                    code: r.status.as_u16(),
                                    content_type: r.headers.get(CONTENT_TYPE).and_then(|h| h.to_str().ok())
                                        .unwrap_or("No content").to_string(),
                                    duration: 100
                                });

                                let (method, line) = match flow.request.as_ref() {
                                    Some(req) => {
                                        (req.method.clone(), req.line_pretty())
                                    },
                                    None => {
                                        (Method::GET, "?????".to_string())
                                    }
                                };

                                flows.push(UiFlow {
                                    id: *id,
                                    method,
                                    uri: line,
                                    response
                                });
                            }
                        }
                        if let Err(e) = ui_tx.send(UiState{ flows }) {
                            error!("error posting ui state {e}");

                        }
                    }
                    _ = shutdown_rx.changed() => {
                        break;
                    }
                }
            }
        })
    }

    fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let len = self.ui_rx.borrow().flows.len();
                if i + 1 < len { i + 1 } else { i }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
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
        let _ = self.shutdown_tx.send(());

        if let Some(handle) = self.listener_handle.take() {
            handle.abort();
        }
    }
}

impl Component for FlowList {
    fn update(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Down => {
                self.next_row();
                ActionResult::Consumed
            }
            Action::Up => {
                self.previous_row();
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let guard = self.ui_rx.borrow_and_update();

        let mut rows = vec![];
        for flow in &guard.flows {
            let c = Line::from(vec![
                Span::styled(
                    flow.method.to_string(),
                    Style::default().fg(method_color(&flow.method)),
                ),
                Span::styled("   ", Style::default()),
                Span::styled(&flow.uri, Style::default().fg(Color::Cyan)),
            ]);
            let l = Row::new(vec![Cell::new(c)]);

            rows.push(l);
            match &flow.response {
                Some(resp) => {
                    rows.push(themed_row!(Line::from(vec![Span::styled(
                        format!(" - {} {} {}", resp.code, resp.content_type, resp.duration),
                        Style::default().fg(method_color(&flow.method))
                    ),])));
                }
                None => rows.push(themed_row!(Line::from("-"))),
            }
        }

        let widths = [Constraint::Fill(1)];

        f.render_stateful_widget(
            themed_table(rows, widths, Some("Flows"), self.focus.get()),
            area,
            &mut self.state,
        );
        f.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            area.inner(Margin::default()),
            &mut self.scroll_state,
        );
        Ok(())
    }
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
