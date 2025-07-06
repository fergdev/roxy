use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tokio::sync::{mpsc::Receiver, watch};
use tracing::debug;

use crate::{
    flow::CertInfo,
    ui::framework::{component::Component, theme::themed_block},
};

pub struct FlowDetailsCerts {
    state: watch::Receiver<UiState>,
    focus: rat_focus::FocusFlag,
}

#[derive(Default, Clone)]
struct UiState {
    data: Vec<CertInfo>,
}

impl FlowDetailsCerts {
    pub fn new(mut cert_rx: Receiver<Option<Vec<CertInfo>>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        tokio::spawn({
            async move {
                while let Some(Some(certs)) = cert_rx.recv().await {
                    ui_tx.send(UiState { data: certs }).unwrap_or_else(|e| {
                        debug!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: rat_focus::FocusFlag::named("FlowCerts"),
        }
    }
}

impl rat_focus::HasFocus for FlowDetailsCerts {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::prelude::Rect {
        ratatui::prelude::Rect::default()
    }
}

impl Component for FlowDetailsCerts {
    fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let data = self.state.borrow_and_update().data.clone();

        if data.is_empty() {
            let empty_text = vec![Line::raw("No certificates found")];
            let block = themed_block(Some("Certificates"), self.focus.get());
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        } else {
            render_cert_list_vertical(f, area, &data);
        }

        Ok(())
    }
}

pub fn render_cert_list_vertical(f: &mut Frame<'_>, area: Rect, certs: &[CertInfo]) {
    let constraints = vec![Constraint::Min(6); certs.len()];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, cert) in certs.iter().enumerate() {
        let lines = vec![
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(Color::Yellow)),
                Span::raw(cert.version.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Serial: ", Style::default().fg(Color::Yellow)),
                Span::raw(
                    cert.serial
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Signature OID: ", Style::default().fg(Color::Yellow)),
                Span::raw(&cert.signature_oid),
            ]),
            Line::from(vec![
                Span::styled("Issuer: ", Style::default().fg(Color::Yellow)),
                Span::raw(&cert.issuer),
            ]),
            Line::from(vec![
                Span::styled("Subject: ", Style::default().fg(Color::Yellow)),
                Span::raw(&cert.subject),
            ]),
            Line::from(vec![
                Span::styled("Not Before: ", Style::default().fg(Color::Yellow)),
                Span::raw(&cert.not_before),
            ]),
            Line::from(vec![
                Span::styled("Not After: ", Style::default().fg(Color::Yellow)),
                Span::raw(&cert.not_after),
            ]),
            Line::from(vec![
                Span::styled("Public Key: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("[{} bytes]", cert.public_key.len())),
            ]),
            Line::from(vec![
                Span::styled("Signature: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("[{} bytes]", cert.signature_value.len())),
            ]),
        ];

        let block = Block::default()
            .title(format!("Certificate {}", i + 1))
            .borders(Borders::ALL);

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, chunks[i]);
    }
}
