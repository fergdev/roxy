use crossterm::event::MouseEvent;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use roxy_shared::cert::CapturedResolveClientCert;

use crate::{
    action::Action,
    ui::framework::{
        component::{Component, DispatchCancellation, DispatchResult},
        scroll::TwoAxisScrollState,
        theme::themed_block,
    },
};

pub(crate) struct ServerResolveClientCertComponent {
    area: Rect,
    focus: FocusFlag,
    data: Option<CapturedResolveClientCert>,
    scroll: TwoAxisScrollState,
}

impl ServerResolveClientCertComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ServerResolve"),
            data: None,
            scroll: TwoAxisScrollState::default(),
        }
    }

    pub(crate) fn set_state(&mut self, resolve_client_cert: &Option<CapturedResolveClientCert>) {
        self.data = resolve_client_cert.clone();
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

        let width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        self.scroll.set(
            (width as u16, lines.len() as u16),
            (area.width, area.height),
        );

        let paragraph = Paragraph::new(lines)
            .block(themed_block(Some("Resolve client cert"), self.focus.get()))
            .scroll(self.scroll.offset());
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

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        self.area = area;
        self.render_resolve_client_cert(frame, area);
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
