use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, ScrollbarState, Wrap},
};
use roxy_shared::cert::CapturedResolveClientCert;

use crate::ui::framework::{component::Component, theme::themed_block};

pub(crate) struct ServerResolveClientCertComponent {
    area: Rect,
    focus: FocusFlag,

    data: Option<CapturedResolveClientCert>,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
}

impl ServerResolveClientCertComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ServerResolve"),
            data: None,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, resolve_client_cert: Option<CapturedResolveClientCert>) {
        self.data = resolve_client_cert;
    }

    fn render_resolve_client_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = vec![];

        match &self.data {
            Some(capture) => {
                lines.push(Line::from(Span::styled(
                    "root_hint_subjects",
                    Style::default().bold(),
                )));
                if capture.root_hint_subjects.is_empty() {
                    lines.push("Empty".into());
                } else {
                    capture
                        .root_hint_subjects
                        .iter()
                        .for_each(|s| lines.push(s.to_owned().into()));
                }

                lines.push(Line::from(ratatui::text::Span::styled(
                    "sigschemes",
                    Style::default().bold(),
                )));
                if capture.sigschemes.is_empty() {
                    lines.push("Empty".into());
                } else {
                    capture
                        .sigschemes
                        .iter()
                        .for_each(|s| lines.push(format!("{s:?}").into()));
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
            .block(themed_block(Some("Resolve client cert"), self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ));
        frame.render_widget(paragraph, area);
    }
}

impl Component for ServerResolveClientCertComponent {
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        self.render_resolve_client_cert(frame, area);
        Ok(())
    }
}

impl HasFocus for ServerResolveClientCertComponent {
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
