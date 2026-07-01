use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Clear, Paragraph, Wrap},
};
use tokio::sync::watch;
use tracing::error;

use crate::ui::{
    flow::{details::FlowWatch, line::LineComponent},
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
    area: Rect,
    state_handle: tokio::task::JoinHandle<()>,
}

impl FlowDetailsRequest {
    pub fn new(mut flow_watch: FlowWatch) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let flow_headers = FlowDetailsHeaders::new(flow_watch.clone(), true);
        let body = FlowDetailsBody::new(flow_watch.clone(), true);

        let state_handle = flow_watch.watch(move |flow| {
            if let Some(flow) = flow
                && let Some(req) = flow.request.as_ref()
            {
                if let Err(error) = ui_tx.send(UiState {
                    line_data: req.line_pretty(),
                }) {
                    error!("Failed to send UI state update: {}", error);
                };
                return;
            }
            if let Err(error) = ui_tx.send(UiState {
                line_data: "No data".to_string(),
            }) {
                error!("Failed to send UI state update: {}", error);
            };
        });

        Self {
            focus: FocusFlag::new().with_name("FlowRequest"),
            line_component: LineComponent::new("ResponseLine"),
            ui_state: ui_rx,
            headers: flow_headers,
            body,
            area: Rect::default(),
            state_handle,
        }
    }
}
impl Drop for FlowDetailsRequest {
    fn drop(&mut self) {
        self.state_handle.abort();
    }
}

impl HasFocus for FlowDetailsRequest {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(&self.line_component);
        builder.leaf_widget(&self.headers);
        builder.leaf_widget(&self.body);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

impl Component for FlowDetailsRequest {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.line_component, &mut self.headers, &mut self.body]
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
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

        self.headers.render(frame, chunks[1]);
        self.body.render(frame, chunks[2]);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
