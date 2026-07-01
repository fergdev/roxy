use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Clear,
};

use roxy_proxy::{
    flow::{FlowCerts, InterceptedRequest, InterceptedResponse, Timing, WsMessage},
    flow_store::FlowStore,
};
use strum::EnumIter;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::error;

use crate::ui::{
    framework::tab::TabComponent,
    framework::{component::Component, util::centered_rect},
};

use super::response::FlowDetailsResponse;
use super::{certs::FlowDetailsCerts, timing::FlowTiming};
use super::{request::FlowDetailsRequest, ws_details::FlowDetailsWs};

#[derive(Default, EnumIter, Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Request,
    Response,
    Certs,
    Timing,
    Ws,
}

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
}

pub struct FlowDetails {
    focus: FocusFlag,
    area: Rect,
    tab_component: TabComponent,
    selected_flow: Option<i64>,
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
            focus: FocusFlag::new().with_name("FlowDetails"),
            area: Rect::default(),
            tab_component: TabComponent::new(
                "Details".to_string(),
                Tab::all()
                    .iter()
                    .map(|t| t.title().to_string())
                    .collect::<Vec<_>>(),
            ),
            selected_flow: None,
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
        self.tab_component.focus.set(true);
        self.selected_flow = Some(flow_id);
        self.flow_id_tx.send(Some(flow_id)).unwrap_or_else(|_| {
            error!("Failed to send flow ID, channel closed");
        });
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

impl HasFocus for FlowDetails {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        let tag = builder.start(self);

        builder.widget(&self.tab_component);
        let tab = Tab::all()[self.tab_component.current_tab];
        let widget: &dyn HasFocus = match tab {
            Tab::Request => &self.request,
            Tab::Response => &self.response,
            Tab::Certs => &self.certs,
            Tab::Timing => &self.timing,
            Tab::Ws => &self.ws,
        };
        builder.widget(widget);
        builder.end(tag);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetails {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        let tab = Tab::all()[self.tab_component.current_tab];
        let selected_child: &mut dyn Component = match tab {
            Tab::Request => &mut self.request,
            Tab::Response => &mut self.response,
            Tab::Certs => &mut self.certs,
            Tab::Timing => &mut self.timing,
            Tab::Ws => &mut self.ws,
        };
        vec![&mut self.tab_component, selected_child]
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.area = area;

        let popup_area = centered_rect(100, 100, area);

        frame.render_widget(Clear, popup_area);

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(popup_area);

        self.tab_component.render(frame, layout[0]);

        let tab = Tab::all()[self.tab_component.current_tab];
        let component: &mut dyn Component = match tab {
            Tab::Request => &mut self.request,
            Tab::Response => &mut self.response,
            Tab::Certs => &mut self.certs,
            Tab::Timing => &mut self.timing,
            Tab::Ws => &mut self.ws,
        };
        component.render(frame, layout[1]);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}

impl Drop for FlowDetails {
    fn drop(&mut self) {
        self.listener_handle.abort();
    }
}
