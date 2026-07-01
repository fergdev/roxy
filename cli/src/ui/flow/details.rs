use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Clear,
};

use roxy_proxy::{flow::Flow, flow_store::FlowStore};
use strum::EnumIter;
use tokio::{sync::watch, task::JoinHandle};
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

#[derive(Debug, Clone)]
pub(crate) struct FlowWatch {
    flow_id_rx: watch::Receiver<Option<i64>>,
    flow_store: FlowStore,
}

impl FlowWatch {
    pub(crate) fn watch(
        &mut self,
        mut on_update: impl FnMut(Option<&Flow>) + Send + 'static,
    ) -> JoinHandle<()> {
        let flow_store = self.flow_store.clone();
        let mut flow_id_rx = self.flow_id_rx.clone();
        tokio::spawn(async move {
            loop {
                let _ = flow_id_rx.changed().await;
                let flow_id = flow_id_rx.borrow_and_update();
                let Some(flow) = flow_id.as_ref() else {
                    on_update(None);
                    continue;
                };
                let Some(flow) = flow_store.flows.get(flow) else {
                    on_update(None);
                    continue;
                };
                if let Ok(flow) = flow.try_read() {
                    on_update(Some(&flow));
                }
            }
        })
    }
}

pub struct FlowDetails {
    focus: FocusFlag,
    area: Rect,
    tab_component: TabComponent,
    selected_flow: Option<i64>,
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

        let flow_watch = FlowWatch {
            flow_id_rx: rx.clone(),
            flow_store: flow_store.clone(),
        };

        let request = FlowDetailsRequest::new(flow_watch.clone());
        let response = FlowDetailsResponse::new(flow_watch.clone());
        let certs = FlowDetailsCerts::new(flow_watch.clone());
        let timing = FlowTiming::new(flow_watch.clone());
        let ws = FlowDetailsWs::new(flow_watch.clone());

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
        self.flow_id_tx.send(Some(flow_id)).unwrap_or_else(|e| {
            error!("Failed to send flow ID '{e}'");
        });
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
