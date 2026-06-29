use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, ScrollbarState, Wrap},
};
use roxy_shared::cert::{ClientVerificationCapture, TlsVerify};

use crate::ui::{
    flow::certs::{CertInfo, render_cert},
    framework::{component::Component, theme::themed_block},
};

pub struct ClientCertComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<ClientVerificationCapture>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}

impl ClientCertComponent {
    pub fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("ClientHelloComponent"),
            area: Rect::default(),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, hello: &Option<ClientVerificationCapture>) {
        self.data = hello.clone();
    }

    fn render_client_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = vec![];

        match &self.data {
            Some(capture) => {
                lines.push("Capture".into());
                match &capture.cert {
                    Some(cert) => {
                        lines.push("End entity".into());

                        match CertInfo::from_der(cert.end_entity.clone()) {
                            Some(ci) => {
                                render_cert(&ci, &mut lines);
                            }
                            None => {
                                lines.push("Failed to render cert".into());
                            }
                        }

                        for aaa in &cert.intermediates {
                            match CertInfo::from_der(aaa.clone()) {
                                Some(ci) => {
                                    render_cert(&ci, &mut lines);
                                }
                                None => {
                                    lines.push("Failed to render cert".into());
                                }
                            }
                        }
                        lines.push("End entity".into());
                    }
                    None => {
                        lines.push("No certs".into());
                    }
                }

                match &capture.tls {
                    TlsVerify::Tls13(tls_capture) => lines.push(format!("{tls_capture:?}").into()),
                    TlsVerify::Tls12(tls_capture) => lines.push(format!("{tls_capture:?}").into()),
                    TlsVerify::None => lines.push("No tls data".into()),
                }
            }
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
    }
}

impl Component for ClientCertComponent {
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        self.render_client_cert(frame, area);
        Ok(())
    }
}

impl HasFocus for ClientCertComponent {
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
