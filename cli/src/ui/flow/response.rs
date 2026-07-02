use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Paragraph, Wrap},
};
use tokio::sync::watch::{self};

use crate::ui::{
    flow::{details::FlowWatch, line::LineComponent},
    framework::{
        component::Component,
        theme::{themed_block, with_theme},
    },
};

use super::{body::component::FlowDetailsBody, headers::FlowDetailsHeaders};

#[derive(Default, Clone)]
enum UiState {
    #[default]
    None,
    ResponseLine(String),
    Error(String),
}

pub struct FlowDetailsResponse {
    focus: FocusFlag,
    ui_state: watch::Receiver<UiState>,
    line_component: LineComponent,
    headers: FlowDetailsHeaders,
    body: FlowDetailsBody,
    area: Rect,
    state_handle: tokio::task::JoinHandle<()>,
}

impl FlowDetailsResponse {
    pub fn new(mut flow_watch: FlowWatch) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let flow_headers = FlowDetailsHeaders::new(flow_watch.clone(), false);
        let body = FlowDetailsBody::new(flow_watch.clone(), false);

        let state_handle = flow_watch.watch(move |flow| {
            let flow = match flow {
                Some(flow) => flow,
                None => {
                    let _ = ui_tx.send(UiState::Error("No flow data".to_string()));
                    return;
                }
            };
            if let Some(response) = flow.response.as_ref() {
                let _ = ui_tx.send(UiState::ResponseLine(response.request_line()));
                return;
            }
            if let Some(error) = flow.error.as_ref() {
                let _ = ui_tx.send(UiState::Error(format!("Error: {}", error)));
                return;
            }
            let _ = ui_tx.send(UiState::Error("No response data".to_string()));
        });

        Self {
            focus: FocusFlag::new().with_name("FlowResponse"),
            ui_state: ui_rx,
            line_component: LineComponent::new("ResponseLine"),
            headers: flow_headers,
            area: Rect::default(),
            body,
            state_handle,
        }
    }
}

impl Drop for FlowDetailsResponse {
    fn drop(&mut self) {
        self.state_handle.abort();
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

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;

        let state = self.ui_state.borrow_and_update();

        let paragraph = match *state {
            UiState::ResponseLine(ref line) => Paragraph::new(Span::from(line))
                .block(themed_block(Some("Line"), self.line_component.focus.get()))
                .wrap(Wrap { trim: true }),
            UiState::Error(ref err) => with_theme(|theme| {
                Paragraph::new(Span::styled(err, Style::default().fg(theme.colors.error)))
                    .block(themed_block(Some("Error"), self.line_component.focus.get()))
                    .wrap(Wrap { trim: true })
            }),
            UiState::None => Paragraph::new(Span::from("No data"))
                .block(themed_block(Some("Line"), self.line_component.focus.get()))
                .wrap(Wrap { trim: true }),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Min(0),
            ])
            .split(area);

        frame.render_widget(paragraph, chunks[0]);

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
