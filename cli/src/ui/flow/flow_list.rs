use std::sync::{Arc, Mutex};

use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, TableState},
};
use roxy_proxy::flow::{FlowKind, FlowStore};
use tokio::{sync::watch, task::JoinHandle};

use crate::{
    app::ITEM_HEIGHT,
    event::Action,
    themed_row,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_table,
    },
};

struct UiFlow {
    pub id: i64,
    pub line: String,
}

#[derive(Clone)]
struct UiState {
    flows: Arc<Mutex<Vec<UiFlow>>>,
}

pub struct FlowList {
    focus: FocusFlag,
    flow_store: FlowStore,
    state: TableState,
    scroll_state: ScrollbarState,
    ui_state: UiState,
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

        let mut instance = Self {
            focus: FocusFlag::named("FlowList"),
            flow_store,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            ui_state: UiState {
                flows: Arc::new(Mutex::new(Vec::new())),
            },
            listener_handle: None,
            shutdown_tx,
        };

        let handle = instance.start_listener(shutdown_rx);
        instance.listener_handle = Some(handle);

        instance
    }

    fn start_listener(&self, mut shutdown_rx: watch::Receiver<()>) -> tokio::task::JoinHandle<()> {
        let flow_store = self.flow_store.clone();
        let ui_state = self.ui_state.clone();

        tokio::spawn(async move {
            let mut flow_rx = flow_store.subscribe();

            loop {
                tokio::select! {
                    _ = flow_rx.changed() => {
                        let ids = flow_store.ordered_ids.read().await;

                        let mut rows = Vec::new();
                        for id in ids.iter() {
                            if let Some(entry) = flow_store.flows.get(id) {

                                let flow = entry.value().read().await;
                                match &flow.kind {
                                    FlowKind::Https(flow) => {
                                        let c = if flow.response.is_some() { "+" } else { "-" };
                                        let resp = flow
                                            .response
                                            .as_ref()
                                            .map(|r| r.status.to_string())
                                            .unwrap_or_else(|| "-".into());
                                        let req = flow
                                            .request.line_pretty();

                                        rows.push(UiFlow {
                                            id: *id,
                                            line: format!("{c} {req} -> {resp}"),
                                        });
                                    }
                                    FlowKind::Http(flow) => {
                                        let c = if flow.response.is_some() { "+" } else { "-" };
                                        let resp = flow
                                            .response
                                            .as_ref()
                                            .map(|r| r.status.to_string())
                                            .unwrap_or_else(|| "-".into());
                                        let req = flow
                                            .request.line_pretty();
                                        rows.push(UiFlow {
                                            id: *id,
                                            line: format!("{c} {req} -> {resp}"),
                                        });
                                    }
                                    FlowKind::Ws(_) => {
                                        rows.push(UiFlow {
                                            id: *id,
                                            line: "web socket".to_owned()
                                        });
                                    }
                                    FlowKind::Wss(_) => {
                                        rows.push(UiFlow {
                                            id: *id,
                                            line: "WSS".to_owned()
                                        });
                                    }
                                    FlowKind::Http2(flow) => {
                                        rows.push(UiFlow {
                                            id: *id,
                                            line: flow.request.line_pretty()
                                        });
                                    }
                                    FlowKind::Unknown => {
                                        let s = match &flow.error {
                                            Some(err) => {
                                                format!("Unknown {err}")
                                            }
                                            None => {
                                                "Unknown".to_string()
                                            }
                                        };
                                        let conn = match &flow.connect {
                                            Some(conn)=>{
                                                conn.line_pretty()
                                            }
                                            None => {
                                                "No-conn".to_string()
                                            }

                                        };

                                        let s = format!("{conn} :{s}");

                                        rows.push(UiFlow {
                                            id: *id,
                                            line: s
                                        });
                                    }
                                }
                            }
                        }
                        let mut flows = ui_state.flows.lock().unwrap();
                        flows.clear();
                        flows.extend(rows);
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
                let len = self.ui_state.flows.lock().unwrap().len();
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
        let guard = self.ui_state.flows.lock().unwrap();
        if let Some(selected) = self.state.selected() {
            if selected < guard.len() {
                Some(guard[selected].id)
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
        let guard = self.ui_state.flows.lock().unwrap();

        let rows = guard.iter().map(|f| themed_row!(vec![f.line.clone()]));

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
