mod client;
mod client_certs;
mod client_hello;
mod client_tls;
mod server;
mod server_certs;
mod server_resolve_client_cert;
mod server_tls;

use bytes::Bytes;
use color_eyre::eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
};
use roxy_proxy::flow::FlowCerts;
use strum::EnumIter;
use tokio::{
    sync::{mpsc::Receiver, watch},
    task::JoinHandle,
};
use tracing::{info, warn};
use x509_parser::parse_x509_certificate;

use crate::{
    action::Action,
    ui::{
        flow::certs::{
            client::{ClientCertificateComponent, ClientState},
            server::{ServerCertificateComponent, ServerState},
        },
        framework::{component::Component, tab::TabComponent, theme::tertiary_text},
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
    focus: FocusFlag,
    area: Rect,

    root_tab_cmp: TabComponent,
    client_cmp: ClientCertificateComponent,
    server_cmp: ServerCertificateComponent,
    state_handle: JoinHandle<()>,
}

impl Drop for FlowDetailsCerts {
    fn drop(&mut self) {
        self.state_handle.abort();
    }
}

#[derive(Default, Clone)]
struct UiState {
    client: ClientState,
    server: ServerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum RootTab {
    Client,
    Server,
}

impl RootTab {
    fn all() -> &'static [RootTab] {
        &[Self::Client, Self::Server]
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Client => "Client",
            Self::Server => "Server",
        }
    }
}

impl FlowDetailsCerts {
    pub fn new(mut cert_rx: Receiver<FlowCerts>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let state_handle = tokio::spawn({
            async move {
                info!("waiting on cert updates...");
                while let Some(certs) = cert_rx.recv().await {
                    let client = ClientState {
                        hello: certs.client_hello.clone(),
                        certs: certs.client_verification,
                        tls: certs.client_tls,
                    };
                    let server = ServerState {
                        resolve_client_cert: certs.server_resolve_client_cert.clone(),
                        certs: certs.server_verification,
                        tls: certs.server_tls,
                    };
                    ui_tx.send(UiState { client, server }).unwrap_or_else(|e| {
                        warn!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });
        Self {
            state: ui_rx,
            focus: FocusFlag::new().with_name("FlowCerts"),
            area: Rect::default(),
            root_tab_cmp: TabComponent::new(
                "Certs".to_string(),
                RootTab::all()
                    .iter()
                    .map(|v| v.title().to_string())
                    .collect(),
            ),
            client_cmp: ClientCertificateComponent::new(),
            server_cmp: ServerCertificateComponent::new(),
            state_handle,
        }
    }
}

impl Component for FlowDetailsCerts {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        let root_tab = RootTab::all()[self.root_tab_cmp.current_tab];
        if matches!(root_tab, RootTab::Client) {
            vec![&mut self.root_tab_cmp, &mut self.client_cmp]
        } else {
            vec![&mut self.root_tab_cmp, &mut self.server_cmp]
        }
    }

    fn handle_tui_event(&mut self, tui_event: crate::tui::TuiEvent) -> Result<Option<Action>> {
        if tui_event == crate::tui::TuiEvent::Render {
            let state = self.state.borrow_and_update();
            if state.has_changed() {
                self.client_cmp.set_state(&state.client);
                self.server_cmp.set_state(&state.server);
            }
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        self.root_tab_cmp.render(frame, layout[0]);
        let root_tab = RootTab::all()[self.root_tab_cmp.current_tab];
        match root_tab {
            RootTab::Client => self.client_cmp.render(frame, layout[1]),
            RootTab::Server => self.server_cmp.render(frame, layout[1]),
        }
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}

impl HasFocus for FlowDetailsCerts {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.root_tab_cmp);
        let root_tab = RootTab::all()[self.root_tab_cmp.current_tab];
        match root_tab {
            RootTab::Client => builder.widget(&self.client_cmp),
            RootTab::Server => builder.widget(&self.server_cmp),
        };
        builder.end(tag);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

fn render_cert<'a>(cert: &CertInfo, lines: &mut Vec<Line<'a>>) {
    lines.push(Line::from(vec![
        Span::styled("Version: ", tertiary_text()),
        Span::raw(cert.version.to_string()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Serial: ", tertiary_text()),
        Span::raw(
            cert.serial
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Signature OID: ", tertiary_text()),
        Span::raw(cert.signature_oid.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Issuer: ", tertiary_text()),
        Span::raw(cert.issuer.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Subject: ", tertiary_text()),
        Span::raw(cert.subject.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Not Before: ", tertiary_text()),
        Span::raw(cert.not_before.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Not After: ", tertiary_text()),
        Span::raw(cert.not_after.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Public Key: ", tertiary_text()),
        Span::raw(format!("[{} bytes]", cert.public_key.len())),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Signature: ", tertiary_text()),
        Span::raw(format!("[{} bytes]", cert.signature_value.len())),
    ]));

    if let Some(san) = &cert.san {
        lines.push(Line::from(vec![
            Span::styled("SAN: ", tertiary_text()),
            Span::raw(san.to_owned()),
        ]))
    }
    if let Some(issuer_cn) = &cert.issuer_cn {
        lines.push(Line::from(vec![
            Span::styled("Iussuer: ", tertiary_text()),
            Span::raw(issuer_cn.to_owned()),
        ]))
    }
    if let Some(subject_cn) = &cert.subject_cn {
        lines.push(Line::from(vec![
            Span::styled("Iussuer: ", tertiary_text()),
            Span::raw(subject_cn.to_owned()),
        ]))
    }
}
