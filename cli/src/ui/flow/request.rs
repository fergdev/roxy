use color_eyre::eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Clear, Paragraph, Wrap},
};
use roxy_proxy::flow::InterceptedRequest;
use roxy_shared::content::content_type;
use tokio::sync::{mpsc, watch};
use tracing::{debug, trace};

use crate::ui::{
    flow::tab::LineComponent,
    framework::{component::Component, theme::themed_block},
};

use super::{body::component::FlowDetailsBody, headers::FlowDetailsHeaders};

#[derive(Default, Clone)]
struct UiState {
    line_data: String,
}

pub struct FlowDetailsRequest {
    focus: FocusFlag,
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
            focus: FocusFlag::new().with_name("FlowRequest"),
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
                                line_data: req.line_pretty(),
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

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetailsRequest {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.line_component, &mut self.headers, &mut self.body]
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) -> Result<()> {
        let data = self.ui_state.borrow_and_update();

        let para = Paragraph::new(Span::from(&data.line_data))
            .block(themed_block(Some("Line"), self.line_component.focus.get()))
            .wrap(Wrap { trim: false });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Min(0),
            ])
            .split(area);

        frame.render_widget(Clear, chunks[0]);
        frame.render_widget(para, chunks[0]);

        self.headers.render(frame, chunks[1])?;
        self.body.render(frame, chunks[2])?;

        Ok(())
    }
}
