use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect, widgets::ScrollbarState};
use roxy_shared::cert::ServerTlsConnectionData;

use crate::ui::framework::{
    component::Component,
    paragraph::kv_paragraph,
    scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
};

pub(crate) struct ClientTlsComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<Vec<(String, String)>>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}
impl ClientTlsComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ClientTlsComponent"),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    pub fn set_state(&mut self, tls: Option<ServerTlsConnectionData>) {
        self.data = Some(process_client_tls(&tls));
    }
}

impl Component for ClientTlsComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        if let Some(data) = &self.data {
            kv_paragraph(data, frame, area, Some("TLS"), self.focus.get(), (0, 0));
            render_vertical_scrollbar(frame, area, &mut self.scroll_index_vertical);
            render_horizontal_scrollbar(frame, area, &mut self.scroll_index_horizontal);
        } else {
            frame.render_widget(
                ratatui::widgets::Paragraph::new("No data")
                    .block(ratatui::widgets::Block::default().title("Hello")),
                area,
            );
        }
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}

impl HasFocus for ClientTlsComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}

fn process_client_tls(data: &Option<ServerTlsConnectionData>) -> Vec<(String, String)> {
    let mut lines: Vec<(String, String)> = vec![];

    match data {
        Some(capture) => {
            lines.push((
                "protocol_version".to_string(),
                match capture.protocol_version {
                    Some(version) => format!("{version:?}"),
                    None => "None".to_string(),
                },
            ));

            lines.push((
                "cipher_suite".to_string(),
                match capture.cipher_suite {
                    Some(cipher_suite) => format!("{cipher_suite:?}"),
                    None => "None".to_string(),
                },
            ));

            lines.push(("sni".to_string(), format!("{:?}", capture.sni)));

            lines.push((
                "key_exchange_group".to_string(),
                match &capture.key_exchange_group {
                    Some(key_exchange_group) => format!("{key_exchange_group:?}"),
                    None => "None".to_string(),
                },
            ));
            lines.push(("alpn".to_string(), format!("{:?}", capture.alpn)));
        }
        None => {
            lines.push(("No data".to_string(), String::new()));
        }
    }
    lines
}
