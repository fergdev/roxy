use crossterm::event::MouseEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use roxy_shared::cert::ServerVerificationCapture;

use crate::{
    action::Action,
    ui::{
        flow::certs::{CertInfo, render_cert},
        framework::{
            component::{Component, DispatchCancellation, DispatchResult},
            scroll::TwoAxisScrollState,
            theme::themed_block,
        },
    },
};

pub(crate) struct ServerCertsComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<ServerVerificationCapture>,

    scroll: TwoAxisScrollState,
}

impl ServerCertsComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ServerResolve"),
            data: None,
            scroll: TwoAxisScrollState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, certs: &Option<ServerVerificationCapture>) {
        self.data = certs.clone();
    }
    fn render_server_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = vec![];
        let header_style = Style::default().bold().underlined();

        match self.data.as_ref() {
            Some(capture) => match &capture.cert {
                Some(cert) => {
                    lines.push(Line::from(vec![Span::styled("End entity", header_style)]));
                    match CertInfo::from_der(cert.end_entity.clone()) {
                        Some(cert_info) => {
                            render_cert(&cert_info, &mut lines);
                        }
                        None => {
                            lines.push("Failed to render cert".into());
                        }
                    }
                    for (index, certificate) in cert.intermediates.iter().enumerate() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("Intermediatary {index}"),
                            header_style,
                        )]));
                        match CertInfo::from_der(certificate.clone()) {
                            Some(ci) => {
                                render_cert(&ci, &mut lines);
                            }
                            None => {
                                lines.push("Failed to render cert".into());
                            }
                        }
                    }
                }
                None => {
                    lines.push("No certs".into());
                }
            },
            None => {
                lines.push("No data".into());
            }
        }

        let height = lines.len();
        let width: usize = lines.iter().fold(0, |acc, line| acc.max(line.width()));

        self.scroll.set_content_size((width as u16, height as u16));

        let paragraph = Paragraph::new(lines)
            .block(themed_block(Some("Certs"), self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll(self.scroll.offset());
        frame.render_widget(paragraph, area);

        self.scroll.render(frame, area);
    }
}

impl Component for ServerCertsComponent {
    fn area(&self) -> Rect {
        self.area
    }
    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        self.area = area;
        self.render_server_cert(frame, area);
    }

    fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> DispatchResult {
        if !self.focus.get() {
            return DispatchCancellation::action(Action::FocusReq(self.focus.id()));
        }

        self.scroll.handle_mouse_event(mouse);
        Ok(())
    }
    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        if self.scroll.handle_action(action) {
            DispatchCancellation::stop()
        } else {
            Ok(())
        }
    }
}

impl HasFocus for ServerCertsComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}
