use rat_focus::HasFocus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Wrap},
};
use roxy_proxy::flow::InterceptedResponse;
use roxy_shared::content::content_type;
use tokio::sync::{mpsc, watch};
use tracing::debug;

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

pub struct FlowDetailsResponse {
    focus: rat_focus::FocusFlag,
    ui_state: watch::Receiver<UiState>,
    line_component: LineComponent,
    headers: FlowDetailsHeaders,
    body: FlowDetailsBody,
}

impl FlowDetailsResponse {
    pub fn new(mut req_rx: tokio::sync::mpsc::Receiver<Option<InterceptedResponse>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());
        let (headers_tx, headers_rx) = mpsc::channel(64);
        let (body_tx, body_rx) = mpsc::channel(64);

        let flow_headers = FlowDetailsHeaders::new(headers_rx);
        let body = FlowDetailsBody::new(body_rx);

        let this = Self {
            focus: rat_focus::FocusFlag::named("FlowResponse"),
            ui_state: ui_rx,
            line_component: LineComponent::new("ResponseLine"),
            headers: flow_headers,
            body,
        };

        tokio::spawn({
            async move {
                while let Some(req) = req_rx.recv().await {
                    if let Some(resp) = req {
                        ui_tx
                            .send(UiState {
                                data: resp.request_line(),
                            })
                            .unwrap_or_else(|e| {
                                debug!("Failed to send UI state update: {}", e);
                            });

                        headers_tx
                            .send(resp.headers.clone())
                            .await
                            .unwrap_or_else(|e| {
                                debug!("Failed to send headers: {}", e);
                            });

                        let content_type = content_type(&resp.headers);
                        body_tx
                            .send((content_type, resp.body.clone()))
                            .await
                            .unwrap_or_else(|e| {
                                debug!("Failed to send body: {}", e);
                            });
                    } else {
                        debug!("Received None request");
                    }
                }
            }
        });
        this
    }
}

impl HasFocus for FlowDetailsResponse {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.widget(&self.line_component);
        builder.widget(&self.headers);
        builder.widget(&self.body);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetailsResponse {
    fn update(&mut self, action: Action) -> ActionResult {
        self.headers.update(action.clone());
        self.body.update(action)
    }

    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let state = self.ui_state.borrow_and_update();

        let para = Paragraph::new(Span::from(state.data.clone()))
            .block(themed_block(Some("Line"), self.line_component.focus.get()))
            .wrap(Wrap { trim: true });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Min(0),
            ])
            .split(area);

        f.render_widget(para, chunks[0]);

        self.headers.render(f, chunks[1])?;
        self.body.render(f, chunks[2])?;
        Ok(())
    }
}
