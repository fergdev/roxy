use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use roxy_shared::cert::{ClientVerificationCapture, TlsVerify};

use crate::ui::{
    flow::certs::{CertInfo, render_cert},
    framework::{component::Component, scroll::TwoAxisScrollState, theme::themed_block},
};

pub struct ClientCertComponent {
    area: Rect,
    focus: FocusFlag,
    data: Option<ClientVerificationCapture>,
    scroll: TwoAxisScrollState,
}

impl ClientCertComponent {
    pub fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("ClientHelloComponent"),
            area: Rect::default(),
            data: None,
            scroll: TwoAxisScrollState::default(),
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

        let height = lines.len();
        let width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        self.scroll.set_content_height(height as u16);
        self.scroll.set_content_width(width as u16);

        let paragraph = Paragraph::new(lines)
            .block(themed_block(Some("Certs"), self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll(self.scroll.offset());
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

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        self.render_client_cert(frame, area);
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
