use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Wrap},
};
use roxy_proxy::flow::InterceptedResponse;
use roxy_shared::content::content_type;
use tokio::sync::{mpsc, watch};
use tracing::debug;

use crate::ui::{
    flow::tab::LineComponent,
    framework::{component::Component, theme::themed_block},
};

use super::{body::component::FlowDetailsBody, headers::FlowDetailsHeaders};

#[derive(Default, Clone)]
struct UiState {
    data: String,
}

pub struct FlowDetailsResponse {
    focus: FocusFlag,
    ui_state: watch::Receiver<UiState>,
    line_component: LineComponent,
    headers: FlowDetailsHeaders,
    body: FlowDetailsBody,
    area: Rect,
}

impl FlowDetailsResponse {
    pub fn new(mut req_rx: tokio::sync::mpsc::Receiver<Option<InterceptedResponse>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());
        let (headers_tx, headers_rx) = mpsc::channel(64);
        let (body_tx, body_rx) = mpsc::channel(64);

        let flow_headers = FlowDetailsHeaders::new(headers_rx);
        let body = FlowDetailsBody::new(body_rx);

        let this = Self {
            focus: FocusFlag::new().with_name("FlowResponse"),
            ui_state: ui_rx,
            line_component: LineComponent::new("ResponseLine"),
            headers: flow_headers,
            area: Rect::default(),
            body,
        };

        tokio::spawn({
            async move {
                while let Some(req) = req_rx.recv().await {
                    if let Some(resp) = req {
                        if let Err(error) = ui_tx.send(UiState {
                            data: resp.request_line(),
                        }) {
                            debug!("Failed to send UI state update: {}", error);
                        };

                        if let Err(error) = headers_tx.send(resp.headers.clone()).await {
                            debug!("Failed to send headers: {}", error);
                        }

                        if let Err(error) = body_tx
                            .send((content_type(&resp.headers), resp.body.clone()))
                            .await
                        {
                            debug!("Failed to send body: {}", error);
                        }
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
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetailsResponse {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.line_component, &mut self.headers, &mut self.body]
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        let state = self.ui_state.borrow_and_update();

        let paragraph = Paragraph::new(Span::from(state.data.clone()))
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

        frame.render_widget(paragraph, chunks[0]);

        self.headers.render(frame, chunks[1])?;
        self.body.render(frame, chunks[2])?;
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }
}
