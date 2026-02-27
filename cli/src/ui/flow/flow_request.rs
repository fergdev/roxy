use rat_focus::HasFocus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Clear, Paragraph, Wrap},
};
use roxy_proxy::flow::InterceptedRequest;
use roxy_shared::content::content_type;
use tokio::sync::{mpsc, watch};
use tracing::{debug, trace};

use crate::{
    event::Action,
    ui::{
        flow::tab::LineComponent,
        framework::{
            component::{ActionResult, Component},
            theme::themed_block,
        },
    },
};

use super::{flow_body::FlowDetailsBody, flow_headers::FlowDetailsHeaders};

#[derive(Default, Clone)]
struct UiState {
    data: String,
}

pub struct FlowDetailsRequest {
    focus: rat_focus::FocusFlag,
    ui_state: watch::Receiver<UiState>,
    line_component: LineComponent,
    headers: FlowDetailsHeaders,
    body: FlowDetailsBody,
}

impl FlowDetailsRequest {
    pub fn new(mut req_rx: tokio::sync::mpsc::Receiver<Option<InterceptedRequest>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());
        let (headers_tx, headers_rx) = mpsc::channel(64);
        let (body_tx, body_rx) = mpsc::channel(64);

        let flow_headers = FlowDetailsHeaders::new(headers_rx);
        let body = FlowDetailsBody::new(body_rx);

        let this = Self {
            focus: rat_focus::FocusFlag::new().with_name("FlowRequest"),
            line_component: LineComponent::new("ResponseLine"),
            ui_state: ui_rx,
            headers: flow_headers,
            body,
        };

        tokio::spawn({
            async move {
                while let Some(req) = req_rx.recv().await {
                    if let Some(req) = req {
                        ui_tx
                            .send(UiState {
                                data: req.line_pretty(),
                            })
                            .unwrap_or_else(|e| {
                                debug!("Failed to send UI state update: {}", e);
                            });

                        headers_tx
                            .send(req.headers.clone())
                            .await
                            .unwrap_or_else(|e| {
                                debug!("Failed to send headers: {}", e);
                            });

                        let content_type = content_type(&req.headers);
                        body_tx
                            .send((content_type, req.body.clone()))
                            .await
                            .unwrap_or_else(|e| {
                                debug!("Failed to send body: {}", e);
                            });
                        trace!("Received request: {}", req.line_pretty());
                    } else {
                        trace!("Received None request");
                    }
                }
            }
        });

        this
    }
}

impl HasFocus for FlowDetailsRequest {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(&self.line_component);
        builder.leaf_widget(&self.headers);
        builder.leaf_widget(&self.body);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetailsRequest {
    fn update(&mut self, action: Action) -> ActionResult {
        self.headers.update(action.clone());
        self.body.update(action)
    }

    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let data = self.ui_state.borrow_and_update();

        let para = Paragraph::new(Span::from(&data.data))
            .block(themed_block(Some("Line"), self.line_component.focus.get()))
            .wrap(Wrap { trim: false });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Min(0),
            ])
            .split(area);

        f.render_widget(Clear, chunks[0]);
        f.render_widget(para, chunks[0]);

        self.headers.render(f, chunks[1])?;
        self.body.render(f, chunks[2])?;

        Ok(())
    }
}
