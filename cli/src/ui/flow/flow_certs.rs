use bytes::Bytes;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use roxy_proxy::flow::FlowCerts;
use roxy_shared::cert::{
    CapturedClientHello, CapturedResolveClientCert, ClientTlsConnectionData,
    ClientVerificationCapture, ServerTlsConnectionData, ServerVerificationCapture, TlsVerify,
};
use strum::EnumIter;
use tokio::{
    sync::{mpsc::Receiver, watch},
    task::JoinHandle,
};
use tracing::{info, warn};
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
    focus: FocusFlag,
    handle: JoinHandle<()>,
    tab: TabComponent,
    client_tab_cmp: TabComponent,
    server_tab_cmp: TabComponent,
    root_tab: RootTab,
    client_tab: ClientTab,
    server_tab: ServerTab,
    scroll_index: usize,
}

impl Drop for FlowDetailsCerts {
    fn drop(&mut self) {
        self.handle.abort();
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

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            *all_tabs.last().unwrap_or(&Self::Server)
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.first().unwrap_or(&Self::Client)
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum ClientTab {
    Hello,
    Certs,
    Tls,
}

impl ClientTab {
    fn all() -> &'static [ClientTab] {
        &[Self::Hello, Self::Certs, Self::Tls]
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Hello => "Hello",
            Self::Certs => "Certs",
            Self::Tls => "Tls",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            *all_tabs.last().unwrap_or(&Self::Tls)
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.last().unwrap_or(&Self::Hello)
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum ServerTab {
    ResolveClientCert,
    Certs,
    Tls,
}

impl ServerTab {
    fn all() -> &'static [ServerTab] {
        &[Self::ResolveClientCert, Self::Certs, Self::Tls]
    }

    fn title(&self) -> &'static str {
        match self {
            Self::ResolveClientCert => "Resolve",
            Self::Certs => "Certs",
            Self::Tls => "Tls",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            *all_tabs.last().unwrap_or(&Self::ResolveClientCert)
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.first().unwrap_or(&Self::Tls)
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Default, Clone)]
struct ClientState {
    hello: Option<CapturedClientHello>,
    certs: Option<ClientVerificationCapture>,
    tls: Option<ServerTlsConnectionData>,
}

#[derive(Default, Clone)]
struct ServerState {
    resolve_client_cert: Option<CapturedResolveClientCert>,
    certs: Option<ServerVerificationCapture>,
    tls: Option<ClientTlsConnectionData>,
}

impl FlowDetailsCerts {
    pub fn new(mut cert_rx: Receiver<FlowCerts>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let handle = tokio::spawn({
            async move {
                info!("waiting on cert updates...");
                while let Some(certs) = cert_rx.recv().await {
                    info!("REC certs {certs:?}");
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
            handle,
            tab: TabComponent::new("FlowTabCerts"),
            client_tab_cmp: TabComponent::new("ClientTab"),
            server_tab_cmp: TabComponent::new("ServerTab"),
            root_tab: RootTab::Client,
            client_tab: ClientTab::Hello,
            server_tab: ServerTab::ResolveClientCert,
            scroll_index: 0,
        }
    }

    fn render_client(&mut self, f: &mut Frame<'_>, area: Rect) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

        let tab_titles: Vec<Line> = ClientTab::all()
            .iter()
            .map(|v| v.title().into())
            .collect::<_>();

        let tabs = themed_tabs(
            Some("Client"),
            tab_titles,
            self.client_tab.index(),
            self.client_tab_cmp.focus.get(),
        );
        f.render_widget(tabs, layout[0]);
        match self.client_tab {
            ClientTab::Hello => self.render_client_hello(f, layout[1]),
            ClientTab::Certs => self.render_client_cert(f, layout[1]),
            ClientTab::Tls => self.render_client_tls(f, layout[1]),
        }
    }

    fn render_client_hello(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let client_hello = &self.state.borrow().client.hello;
        let mut lines: Vec<Line> = vec![];

        match client_hello {
            Some(capture) => {
                lines.push(Line::from(Span::styled(
                    "Server name",
                    Style::default().bold(),
                )));

                if let Some(server_name) = &capture.server_name {
                    lines.push(server_name.to_owned().into());
                } else {
                    lines.push("None".into());
                }
                lines.push(Line::from(Span::styled(
                    "Signature schemes",
                    Style::default().bold(),
                )));
                if capture.signature_schemes.is_empty() {
                    lines.push("None".into());
                } else {
                    capture
                        .signature_schemes
                        .iter()
                        .for_each(|s| lines.push(format!("{s:?}").into()));
                }
                lines.push(Line::from(Span::styled("ALPN", Style::default().bold())));
                if let Some(alpn) = &capture.alpn {
                    if alpn.is_empty() {
                        lines.push("Empty".into());
                    } else {
                        alpn.iter().for_each(|s| lines.push(s.to_owned().into()));
                    }
                } else {
                    lines.push("None".into());
                }

                // server_cert_types: Option<Vec<String>>,
                lines.push(Line::from(Span::styled(
                    "server_cert_types",
                    Style::default().bold(),
                )));

                if let Some(server_cert_types) = &capture.server_cert_types {
                    if server_cert_types.is_empty() {
                        lines.push("Empty".into());
                    } else {
                        server_cert_types
                            .iter()
                            .for_each(|s| lines.push(s.to_owned().into()));
                    }
                } else {
                    lines.push("None".into());
                }

                // client_cert_types: Option<Vec<String>>,
                lines.push(Line::from(Span::styled(
                    "client_cert_types",
                    Style::default().bold(),
                )));
                if let Some(client_cert_types) = &capture.server_cert_types {
                    if client_cert_types.is_empty() {
                        lines.push("Empty".into());
                    } else {
                        client_cert_types
                            .iter()
                            .for_each(|s| lines.push(s.to_owned().into()));
                    }
                } else {
                    lines.push("None".into());
                }
                // cipher_suites: Vec<String>,
                lines.push(Line::from(Span::styled(
                    "cipher_suites",
                    Style::default().bold(),
                )));

                if capture.cipher_suites.is_empty() {
                    lines.push("Empty".into());
                } else {
                    capture
                        .cipher_suites
                        .iter()
                        .for_each(|s| lines.push(format!("{s:?}").into()));
                }
                // certificate_authorities: Option<Vec<String>>,
                lines.push(Line::from(Span::styled(
                    "certificate_authorities",
                    Style::default().bold(),
                )));
                if let Some(certificate_authorities) = &capture.certificate_authorities {
                    if certificate_authorities.is_empty() {
                        lines.push("Empty".into());
                    } else {
                        certificate_authorities
                            .iter()
                            .for_each(|s| lines.push(s.to_owned().into()));
                    }
                } else {
                    lines.push("None".into());
                }

                // named_groups: Option<Vec<String>>,
                lines.push(Line::from(Span::styled(
                    "named_groups",
                    Style::default().bold(),
                )));
                if let Some(named_groups) = &capture.named_groups {
                    if named_groups.is_empty() {
                        lines.push("Empty".into());
                    } else {
                        named_groups
                            .iter()
                            .for_each(|s| lines.push(s.to_owned().into()));
                    }
                } else {
                    lines.push("None".into());
                }
            }
            None => lines.push("No data".into()),
        }

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_index as u16, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_client_cert(&mut self, f: &mut Frame<'_>, area: Rect) {
        let certs = &self.state.borrow().client.certs;
        let mut lines = vec![];

        match &certs {
            Some(capture) => {
                lines.push("Capture".into());
                match &capture.cert {
                    Some(cert) => {
                        lines.push("End entity".into());

                        match CertInfo::from_der(cert.end_entity.clone()) {
                            Some(ci) => {
                                render_cert(&ci, &mut lines);
                                // f.render_widget(para, area);
                            }
                            None => {
                                lines.push("Failed to render cert".into());
                            }
                        }
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

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    fn render_client_tls(&mut self, f: &mut Frame<'_>, area: Rect) {
        let client_tls = &self.state.borrow().client.tls;
        let mut lines = vec![];

        match client_tls {
            Some(capture) => {
                lines.push(format!("protocol_version: {:?}", capture.protocol_version).into());
                lines.push(format!("cipher_suite: {:?}", capture.cipher_suite).into());
                lines.push(format!("sni: {:?}", capture.sni).into());
                lines.push(format!("key_exchange_group: {:?}", capture.key_exchange_group).into());
                lines.push(format!("alpn: {:?}", capture.alpn).into());
            }
            None => {
                lines.push("No data".into());
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_index as u16, 0));
        f.render_widget(paragraph, area);
    }

    fn render_server(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        let tab_titles: Vec<Line> = ServerTab::all().iter().map(|v| v.title().into()).collect();

        let tabs = themed_tabs(
            None,
            tab_titles,
            self.server_tab.index(),
            self.server_tab_cmp.focus.get(),
        );

        frame.render_widget(tabs, layout[0]);
        match self.server_tab {
            ServerTab::ResolveClientCert => self.render_resolve_client_cert(frame, layout[1]),
            ServerTab::Certs => self.render_server_cert(frame, layout[1]),
            ServerTab::Tls => self.render_server_tls(frame, layout[1]),
        }
    }

    fn render_resolve_client_cert(&mut self, f: &mut Frame<'_>, area: Rect) {
        let certs = &self.state.borrow().server.resolve_client_cert;
        let mut lines = vec![];

        match &certs {
            Some(capture) => {
                // root_hint_subjects
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

                // root_hint_subjects
                lines.push(Line::from(Span::styled(
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

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_index as u16, 0));
        f.render_widget(paragraph, area);
    }

    fn render_server_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let certs = &self.state.borrow().server.certs;
        let mut lines = vec![];

        match certs {
            Some(capture) => {
                lines.push("Capture".into());
                match &capture.cert {
                    Some(cert) => {
                        lines.push("End entity".into());
                        match CertInfo::from_der(cert.end_entity.clone()) {
                            Some(cert_info) => {
                                render_cert(&cert_info, &mut lines);
                            }
                            None => {
                                lines.push("Failed to render cert".into());
                            }
                        }
                    }
                    None => {
                        lines.push("No certs".into());
                    }
                }
            }
            None => {
                lines.push("No data".into());
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_index as u16, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_server_tls(&mut self, f: &mut Frame<'_>, area: Rect) {
        let tls = &self.state.borrow().server.tls;
        let mut lines = vec![];

        match tls {
            Some(capture) => {
                lines.push(format!("protocol_version: {:?}", capture.protocol_version).into());
                lines.push(format!("cipher_suite: {:?}", capture.cipher_suite).into());
                lines.push(format!("ech_status: {:?}", capture.ech_status).into());
                lines.push(format!("key_exchange_group: {:?}", capture.key_exchange_group).into());
                lines.push(format!("alpn: {:?}", capture.alpn).into());
            }
            None => {
                lines.push("No data".into());
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_index as u16, 0));
        f.render_widget(paragraph, area);
    }
}

fn render_cert<'a>(cert: &CertInfo, lines: &mut Vec<Line<'a>>) {
    lines.push(Line::from(vec![
        Span::styled("Version: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.version.to_string()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Serial: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(
            cert.serial
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Signature OID: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.signature_oid.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Issuer: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.issuer.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Subject: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.subject.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Not Before: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.not_before.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Not After: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(cert.not_after.to_owned()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Public Key: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("[{} bytes]", cert.public_key.len())),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Signature: ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("[{} bytes]", cert.signature_value.len())),
    ]));

    if let Some(san) = &cert.san {
        lines.push(Line::from(vec![
            Span::styled("SAN: ", Style::default().fg(Color::Yellow).bold()),
            Span::raw(san.to_owned()),
        ]))
    }
    if let Some(issuer_cn) = &cert.issuer_cn {
        lines.push(Line::from(vec![
            Span::styled("Iussuer: ", Style::default().fg(Color::Yellow).bold()),
            Span::raw(issuer_cn.to_owned()),
        ]))
    }
    if let Some(subject_cn) = &cert.subject_cn {
        lines.push(Line::from(vec![
            Span::styled("Iussuer: ", Style::default().fg(Color::Yellow).bold()),
            Span::raw(subject_cn.to_owned()),
        ]))
    }
}

impl HasFocus for FlowDetailsCerts {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(&self.tab);
        match self.root_tab {
            RootTab::Client => builder.leaf_widget(&self.client_tab_cmp),
            RootTab::Server => builder.leaf_widget(&self.server_tab_cmp),
        };
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl Component for FlowDetailsCerts {
    fn update(&mut self, action: Action) -> ActionResult {
        if self.tab.focus.get() {
            match action {
                Action::Left => {
                    self.root_tab = self.root_tab.prev();
                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.root_tab = self.root_tab.next();
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        if self.client_tab_cmp.focus.get() {
            match action {
                Action::Left => {
                    self.client_tab = self.client_tab.prev();
                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.client_tab = self.client_tab.next();
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        if self.server_tab_cmp.focus.get() {
            match action {
                Action::Left => {
                    self.server_tab = self.server_tab.prev();
                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.server_tab = self.server_tab.next();
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        match action {
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
        ActionResult::Ignored
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        let tab_titles: Vec<Line> = RootTab::all().iter().map(|v| v.title().into()).collect();

        let tabs = themed_tabs(
            None,
            tab_titles,
            self.root_tab.index(),
            self.tab.focus.get(),
        );
        frame.render_widget(tabs, layout[0]);
        match self.root_tab {
            RootTab::Client => self.render_client(frame, layout[1]),
            RootTab::Server => self.render_server(frame, layout[1]),
        }
        Ok(())
    }
}
