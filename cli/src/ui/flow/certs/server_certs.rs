use color_eyre::eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, ScrollbarState, Wrap},
};
use roxy_shared::cert::ServerVerificationCapture;

use crate::{
    action::Action,
    ui::{
        flow::certs::{CertInfo, render_cert},
        framework::{
            component::{ActionResult, Component},
            scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
            theme::themed_block,
        },
    },
};

pub(crate) struct ServerCertsComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<ServerVerificationCapture>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}

impl ServerCertsComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ServerResolve"),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
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

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(lines.len())
            .viewport_content_length(self.area.height as usize);

        let paragraph = Paragraph::new(lines)
            .block(themed_block(Some("Certs"), self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ));
        frame.render_widget(paragraph, area);

        render_vertical_scrollbar(frame, area, &mut self.scroll_index_vertical);
        render_horizontal_scrollbar(frame, area, &mut self.scroll_index_horizontal);
    }
}

impl Component for ServerCertsComponent {
    fn area(&self) -> Rect {
        self.area
    }
    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) -> Result<()> {
        self.area = area;
        self.render_server_cert(frame, area);

        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
        match action {
            Action::Up => {
                self.scroll_index_vertical.prev();
                ActionResult::Consumed
            }
            Action::Down => {
                self.scroll_index_vertical.next();
                ActionResult::Consumed
            }
            _ => ActionResult::Ignored,
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_index_vertical.prev();
                Ok(None)
            }
            MouseEventKind::ScrollDown => {
                self.scroll_index_vertical.next();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

impl HasFocus for ServerCertsComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}
