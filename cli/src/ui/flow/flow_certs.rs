use bytes::Bytes;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use tokio::sync::{mpsc::Receiver, watch};
use tracing::debug;
use x509_parser::parse_x509_certificate;

use crate::{
    event::Action,
    ui::{
        flow::tab::TabComponent,
        framework::{
            component::{ActionResult, Component},
            theme::{themed_block, themed_tabs},
        },
    },
};

#[derive(Clone, Debug)]
struct CertInfo {
    version: u32,
    serial: Vec<u8>,
    signature_oid: String,
    issuer_cn: Option<String>,
    subject_cn: Option<String>,
    san: Option<String>,
    issuer: String,
    subject: String,
    not_before: String,
    not_after: String,
    public_key: Vec<u8>,
    signature_value: Vec<u8>,
}

impl CertInfo {
    pub fn from_der(cert: Bytes) -> Option<Self> {
        let (_, cert) = parse_x509_certificate(cert.as_ref()).ok()?;
        let tbs = &cert.tbs_certificate;

        let subject_cn = cert
            .subject
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .map(|s| s.to_string());
        let issuer_cn = cert
            .issuer
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .map(|s| s.to_string());

        let san = tbs
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|ext| format!("{:?}", ext.value));

        Some(Self {
            version: tbs.version.0,
            serial: tbs.serial.to_bytes_be(),
            signature_oid: tbs.signature.algorithm.to_id_string(),
            subject_cn,
            issuer_cn,
            san,
            issuer: tbs.issuer.to_string(),
            subject: tbs.subject.to_string(),
            not_before: tbs.validity.not_before.to_datetime().to_string(),
            not_after: tbs.validity.not_after.to_datetime().to_string(),
            public_key: tbs.subject_pki.subject_public_key.data.to_vec(),
            signature_value: cert.signature_value.data.to_vec(),
        })
    }
}

pub struct FlowDetailsCerts {
    state: watch::Receiver<UiState>,
    focus: rat_focus::FocusFlag,
    tab: TabComponent,
    tab_index: usize,
    scroll_index: usize,
}

#[derive(Default, Clone)]
struct UiState {
    data: Vec<CertInfo>,
}

impl FlowDetailsCerts {
    pub fn new(mut cert_rx: Receiver<Option<Vec<Bytes>>>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        tokio::spawn({
            async move {
                while let Some(Some(certs)) = cert_rx.recv().await {
                    let mut data = Vec::new();
                    for bytes in certs {
                        if let Some(info) = CertInfo::from_der(bytes) {
                            data.push(info);
                        }
                    }
                    ui_tx.send(UiState { data }).unwrap_or_else(|e| {
                        debug!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });

        Self {
            state: ui_rx,
            focus: rat_focus::FocusFlag::named("FlowCerts"),
            tab: TabComponent::new("FlowTabCerts"),
            tab_index: 0,
            scroll_index: 0,
        }
    }

    fn render_cert_list_vertical(&mut self, f: &mut Frame<'_>, area: Rect, certs: &[CertInfo]) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

        let tab_titles: Vec<Line> = certs
            .iter()
            .enumerate()
            .map(|(i, _)| Line::raw(i.to_string()))
            .collect();
        let tab_index = self.tab_index;

        let tabs = themed_tabs("Certs", tab_titles, tab_index, self.tab.focus.get());
        f.render_widget(tabs, layout[0]);

        let cert = certs.get(tab_index).unwrap();

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(Color::Yellow)),
                Span::raw(cert.version.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Serial: ", Style::default().fg(Color::Yellow)),
                Span::raw(
                    cert.serial
                        .iter()
                        .map(|b| format!("{b:02x}"))
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

        if let Some(san) = &cert.san {
            lines.push(Line::from(vec![
                Span::styled("SAN: ", Style::default().fg(Color::Yellow)),
                Span::raw(san),
            ]))
        }
        if let Some(issuer_cn) = &cert.issuer_cn {
            lines.push(Line::from(vec![
                Span::styled("Iussuer: ", Style::default().fg(Color::Yellow)),
                Span::raw(issuer_cn),
            ]))
        }
        if let Some(subject_cn) = &cert.subject_cn {
            lines.push(Line::from(vec![
                Span::styled("Iussuer: ", Style::default().fg(Color::Yellow)),
                Span::raw(subject_cn),
            ]))
        }

        let paragraph = Paragraph::new(lines)
            .block(themed_block(Some("Info"), self.focus.get()))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, layout[1]);
    }
}

impl rat_focus::HasFocus for FlowDetailsCerts {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(&self.tab);
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
    fn update(&mut self, _action: Action) -> ActionResult {
        if self.tab.focus.get() {
            match _action {
                Action::Left => {
                    if self.tab_index == 0 {
                        self.tab_index = self.state.borrow().data.len() - 1;
                    } else {
                        self.tab_index -= 1;
                    }

                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.tab_index += 1;
                    if self.tab_index == self.state.borrow().data.len() {
                        self.tab_index = 0
                    }
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        } else if self.focus.get() {
            match _action {
                Action::Down => {
                    self.scroll_index += 1;
                    return ActionResult::Consumed;
                }
                Action::Up => {
                    if self.scroll_index > 0 {
                        self.scroll_index -= 1;
                    }
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        ActionResult::Ignored
    }
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
            self.render_cert_list_vertical(f, area, &data);
        }

        Ok(())
    }
}
