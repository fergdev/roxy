use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph, ScrollbarState},
};
use roxy_shared::cert::CapturedClientHello;

use crate::ui::framework::{
    component::Component,
    paragraph::kv_paragraph,
    scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
};

pub struct ClientHelloComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<Vec<(String, String)>>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}

impl ClientHelloComponent {
    pub fn new() -> Self {
        Self {
            focus: FocusFlag::new().with_name("ClientHelloComponent"),
            area: Rect::default(),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, hello: &Option<CapturedClientHello>) {
        self.data = Some(process_client_hello(hello));
    }
}
impl Component for ClientHelloComponent {
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        if let Some(data) = &self.data {
            kv_paragraph(
                data,
                frame,
                area,
                Some("Hello"),
                self.focus.get(),
                (
                    self.scroll_index_vertical.get_position() as u16,
                    self.scroll_index_horizontal.get_position() as u16,
                ),
            );
            render_vertical_scrollbar(frame, area, &mut self.scroll_index_vertical);
            render_horizontal_scrollbar(frame, area, &mut self.scroll_index_horizontal);
        } else {
            frame.render_widget(
                Paragraph::new("No data").block(Block::default().title("Hello")),
                area,
            );
        }
        Ok(())
    }
}

impl HasFocus for ClientHelloComponent {
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

pub(crate) fn process_client_hello(
    client_hello: &Option<CapturedClientHello>,
) -> Vec<(String, String)> {
    let mut lines = vec![];
    match client_hello {
        Some(capture) => {
            lines.push((
                "Server name".to_string(),
                if let Some(server_name) = &capture.server_name {
                    server_name.to_string()
                } else {
                    "None".to_string()
                },
            ));

            lines.push((
                "Signature schemes".to_string(),
                if capture.signature_schemes.is_empty() {
                    "None".to_string()
                } else {
                    capture
                        .signature_schemes
                        .iter()
                        .map(|s| format!("{s:?}").to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                },
            ));
            lines.push((
                "ALPN".to_string(),
                if let Some(alpn) = &capture.alpn {
                    if alpn.is_empty() {
                        "Empty".to_string()
                    } else {
                        alpn.iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                } else {
                    "None".to_string()
                },
            ));

            lines.push((
                "server_cert_types".to_string(),
                if let Some(server_cert_types) = &capture.server_cert_types {
                    if server_cert_types.is_empty() {
                        "Empty".to_string()
                    } else {
                        server_cert_types
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                } else {
                    "None".to_string()
                },
            ));

            lines.push((
                "client_cert_types".to_string(),
                if let Some(client_cert_types) = &capture.server_cert_types {
                    if client_cert_types.is_empty() {
                        "Empty".to_string()
                    } else {
                        client_cert_types
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                } else {
                    "None".to_string()
                },
            ));
            lines.push((
                "cipher_suites".to_string(),
                if capture.cipher_suites.is_empty() {
                    "Empty".to_string()
                } else {
                    capture
                        .cipher_suites
                        .iter()
                        .map(|s| format!("{s:?}").to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                },
            ));
            lines.push((
                "certificate_authorities".to_string(),
                if let Some(certificate_authorities) = &capture.certificate_authorities {
                    if certificate_authorities.is_empty() {
                        "Empty".to_string()
                    } else {
                        certificate_authorities
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                } else {
                    "None".to_string()
                },
            ));

            lines.push((
                "named_groups".to_string(),
                if let Some(named_groups) = &capture.named_groups {
                    if named_groups.is_empty() {
                        "Empty".to_string()
                    } else {
                        named_groups
                            .iter()
                            .map(|s| format!("{s:?}").to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                } else {
                    "None".to_string()
                },
            ));
        }
        None => lines.push(("No data".to_string(), "".to_string())),
    }
    lines
}
