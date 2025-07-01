use std::sync::{Arc, Mutex};

use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState},
};
use tokio::{sync::watch, task::JoinHandle};
use tracing::debug;

use crate::{
    app::{ITEM_HEIGHT, TableColors},
    event::Action,
    flow::FlowStore,
};

use super::component::Component;

struct UiFlow {
    pub id: i64,
    pub line: String,
}

#[derive(Clone)]
struct UiState {
    flows: Arc<Mutex<Vec<UiFlow>>>,
}

pub struct FlowList {
    flow_store: FlowStore,
    state: TableState,
    scroll_state: ScrollbarState,
    ui_state: UiState,
    colors: TableColors,
    shutdown_tx: watch::Sender<()>,
    listener_handle: Option<JoinHandle<()>>,
}

impl FlowList {
    pub fn new(flow_store: FlowStore, colors: TableColors) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        let mut instance = Self {
            flow_store,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            ui_state: UiState {
                flows: Arc::new(Mutex::new(Vec::new())),
            },
            colors,
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
                                let c = if flow.response.is_some() { "+" } else { "-" };
                                let resp = flow
                                    .response
                                    .as_ref()
                                    .map(|r| r.status.to_string())
                                    .unwrap_or_else(|| "-".into());
                                let req = flow
                                    .request
                                    .as_ref()
                                    .map(|r| r.line_pretty())
                                    .unwrap_or_else(|| "empty".into());

                                rows.push(UiFlow {
                                    id: flow.id,
                                    line: format!("{} {} -> {}", c, req, resp),
                                });
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
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Down => {
                debug!("Next row action");
                self.next_row();
                Ok(None)
            }
            Action::Up => {
                debug!("Previous row action");
                self.previous_row();
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);

        let guard = self.ui_state.flows.lock().unwrap();
        let rows = guard.iter().map(|f| Row::new(vec![f.line.clone()]));

        let table = Table::new(rows, [Constraint::Fill(1)])
            .highlight_symbol(">> ")
            .bg(self.colors.buffer_bg)
            .highlight_style(selected_row_style);

        f.render_stateful_widget(table, area, &mut self.state);
        f.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            area.inner(Margin::default()),
            &mut self.scroll_state,
        );
        Ok(())
    }
}
