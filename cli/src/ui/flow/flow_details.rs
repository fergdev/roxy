use color_eyre::Result;
use rat_focus::HasFocus;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Clear,
};

use roxy_proxy::flow::{
    FlowCerts, FlowStore, InterceptedRequest, InterceptedResponse, Timing, WsMessage,
};
use strum::EnumIter;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::error;

use crate::{
    event::Action,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_tabs,
        util::centered_rect,
    },
};

use super::flow_response::FlowDetailsResponse;
use super::{flow_certs::FlowDetailsCerts, flow_timing::FlowTiming};
use super::{flow_request::FlowDetailsRequest, ws_details::FlowDetailsWs};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Request,
    Response,
    Certs,
    Timing,
    Ws,
}

// TODO: strum this?
impl Tab {
    fn all() -> &'static [Tab] {
        &[
            Self::Request,
            Self::Response,
            Self::Certs,
            Self::Timing,
            Self::Ws,
        ]
    }

    fn title(&self) -> &'static str {
        match self {
            Tab::Request => "Request",
            Tab::Response => "Response",
            Tab::Certs => "Certs",
            Tab::Timing => "Timing",
            Tab::Ws => "Ws",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            *all_tabs.first().unwrap_or(&Self::Ws)
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.first().unwrap_or(&Self::Request)
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum FocusedComponent {
    Tabs,
    Headers,
    Body,
}

pub struct FlowDetails {
    focus: rat_focus::FocusFlag,
    tabs: TabComponent,
    selected_flow: Option<i64>,
    tab: Tab,
    listener_handle: JoinHandle<()>,
    flow_id_tx: watch::Sender<Option<i64>>,
    request: FlowDetailsRequest,
    response: FlowDetailsResponse,
    certs: FlowDetailsCerts,
    timing: FlowTiming,
    ws: FlowDetailsWs,
}

impl FlowDetails {
    pub fn new(flow_store: FlowStore) -> Self {
        let (tx, rx) = watch::channel(None::<i64>);

        // TODO: 64 might be a bit high
        let (req_tx, req_rx) = mpsc::channel::<Option<InterceptedRequest>>(64);
        let (resp_tx, resp_rx) = mpsc::channel::<Option<InterceptedResponse>>(64);
        let (cert_tx, cert_rx) = mpsc::channel::<FlowCerts>(64);
        let (timing_tx, timing_rx) = mpsc::channel::<Timing>(64);
        let (ws_tx, ws_rx) = mpsc::channel::<Vec<WsMessage>>(64);

        let request = FlowDetailsRequest::new(req_rx);
        let response = FlowDetailsResponse::new(resp_rx);
        let certs = FlowDetailsCerts::new(cert_rx);
        let timing = FlowTiming::new(timing_rx);
        let ws = FlowDetailsWs::new(ws_rx);

        let task_flow_store = flow_store.clone();
        let handle = tokio::spawn(async move {
            let mut current_flow_id: Option<i64> = None;
            let mut id_rx = rx;

            let mut flow_rx = task_flow_store.subscribe();
            loop {
                tokio::select! {
                    _ = id_rx.changed() => {
                        current_flow_id = *id_rx.borrow_and_update();
                        update_flow_view(&task_flow_store, current_flow_id, &req_tx, &resp_tx, &ws_tx, &cert_tx, &timing_tx).await;
                    }

                    _ = flow_rx.changed() => {
                        if let Some(flow_id) = current_flow_id {
                            update_flow_view(&task_flow_store, Some(flow_id), &req_tx, &resp_tx, &ws_tx, &cert_tx, &timing_tx).await;
                        }
                    }
                }
            }
        });

        Self {
            focus: rat_focus::FocusFlag::named("FlowDetails"),
            tabs: TabComponent::new(),
            selected_flow: None,
            tab: Tab::Request,
            listener_handle: handle,
            flow_id_tx: tx,
            request,
            response,
            certs,
            timing,
            ws,
        }
    }

    pub fn set_flow(&mut self, flow_id: i64) {
        self.focus();

        self.selected_flow = Some(flow_id);
        self.flow_id_tx.send(Some(flow_id)).unwrap_or_else(|_| {
            error!("Failed to send flow ID, channel closed");
        });
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }

    fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
    }
}

async fn update_flow_view(
    store: &FlowStore,
    flow_id_opt: Option<i64>,
    req_tx: &mpsc::Sender<Option<InterceptedRequest>>,
    resp_tx: &mpsc::Sender<Option<InterceptedResponse>>,
    ws_tx: &mpsc::Sender<Vec<WsMessage>>,
    cert_tx: &mpsc::Sender<FlowCerts>,
    timing_tx: &mpsc::Sender<Timing>,
) {
    if let Some(flow_id) = flow_id_opt {
        let maybe_entry = store.get_flow_by_id(flow_id).await;

        if let Some(entry) = maybe_entry {
            let flow = entry.read().await;
            req_tx.send(flow.request.clone()).await.unwrap_or_else(|e| {
                error!("Failed to send request: {}", e);
            });

            resp_tx
                .send(flow.response.clone())
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send response: {}", e);
                });

            let certs = flow.certs.clone();

            cert_tx.send(certs).await.unwrap_or_else(|e| {
                error!("Failed to send certs: {}", e);
            });
            ws_tx.send(flow.messages.clone()).await.unwrap_or_else(|e| {
                error!("Failed to send WebSocket messages: {}", e);
            });
            timing_tx
                .send(flow.timing.clone())
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send timing: {}", e);
                });
        }
    }
}

struct TabComponent {
    focus: rat_focus::FocusFlag,
}

impl TabComponent {
    pub fn new() -> Self {
        Self {
            focus: rat_focus::FocusFlag::named("FlowDetailsTabs"),
        }
    }
}

impl HasFocus for TabComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl HasFocus for FlowDetails {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.tabs);
        match self.tab {
            Tab::Request => {
                builder.widget(&self.request);
            }
            Tab::Response => {
                builder.widget(&self.response);
            }
            Tab::Certs => {
                builder.widget(&self.certs);
            }
            Tab::Timing => {
                builder.widget(&self.timing);
            }
            Tab::Ws => {
                builder.widget(&self.ws);
            }
        }
        builder.end(tag);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetails {
    fn update(&mut self, action: Action) -> ActionResult {
        if self.tabs.focus.get() {
            match action {
                Action::Left => {
                    self.prev_tab();
                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.next_tab();
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        match self.tab {
            Tab::Request => self.request.update(action),
            Tab::Response => self.response.update(action),
            Tab::Certs => self.certs.update(action),
            Tab::Timing => self.timing.update(action),
            Tab::Ws => self.ws.update(action),
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        let popup_area = centered_rect(100, 100, area);

        f.render_widget(Clear, popup_area);

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(popup_area);
        let tab_titles: Vec<Line> = Tab::all().iter().map(|t| Line::raw(t.title())).collect();
        let tab_index = self.tab.index();

        let tabs = themed_tabs(
            Some("Flow details"),
            tab_titles,
            tab_index,
            self.tabs.focus.get(),
        );
        f.render_widget(tabs, layout[0]);

        match self.tab {
            Tab::Request => {
                self.request.render(f, layout[1])?;
            }
            Tab::Response => {
                self.response.render(f, layout[1])?;
            }
            Tab::Certs => {
                self.certs.render(f, layout[1])?;
            }
            Tab::Timing => {
                self.timing.render(f, layout[1])?;
            }
            Tab::Ws => {
                self.ws.render(f, layout[1])?;
            }
        }

        Ok(())
    }
}

impl Drop for FlowDetails {
    fn drop(&mut self) {
        self.listener_handle.abort();
    }
}
