use color_eyre::eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, ratatui::layout::Rect};
use ratatui::widgets::{Block, Paragraph, ScrollbarState};
use roxy_shared::cert::ClientTlsConnectionData;

use crate::ui::framework::{
    component::Component,
    paragraph::kv_paragraph,
    scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
};

pub(crate) struct ServerTlsComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<Vec<(String, String)>>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}

impl ServerTlsComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ServerResolve"),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, tls: Option<ClientTlsConnectionData>) {
        self.data = Some(process_server_tls(&tls));
    }
}

impl Component for ServerTlsComponent {
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) -> Result<()> {
        self.area = area;
        if let Some(data) = &self.data {
            kv_paragraph(
                data,
                frame,
                area,
                Some("Tls"),
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

impl HasFocus for ServerTlsComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

pub(crate) fn process_server_tls(data: &Option<ClientTlsConnectionData>) -> Vec<(String, String)> {
    let mut lines = vec![];

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

            lines.push((
                "ech_status".to_string(),
                format!("{:?}", capture.ech_status),
            ));

            lines.push((
                "key_exchange_group".to_string(),
                match &capture.key_exchange_group {
                    Some(key_exchange_group) => format!("{key_exchange_group:?}"),
                    None => "None".to_string(),
                },
            ));
            lines.push(("alpn".to_string(), format!("{:?}", &capture.alpn)));
        }
        None => {
            lines.push(("No data".to_string(), "".to_string()));
        }
    }
    lines
}
