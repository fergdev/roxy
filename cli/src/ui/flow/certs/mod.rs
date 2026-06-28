mod client;
mod server;
use bytes::Bytes;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, ScrollbarState, Wrap},
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
    action::Action,
    ui::{
        flow::{
            certs::{
                client::{process_client_hello, process_client_tls},
                server::process_server_tls,
            },
            tab::TabComponent,
        },
        framework::{
            component::{ActionResult, Component},
            paragraph::kv_paragraph,
            scrollbar::{render_horizontal_scrollbar, render_vertical_scrollbar},
            theme::{tertiary_text, themed_block, themed_tabs},
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
    area: Rect,
    handle: JoinHandle<()>,

    tab: TabComponent,
    client_tab_cmp: TabComponent,
    server_tab_cmp: TabComponent,

    root_tab: RootTab,
    client_tab: ClientTab,
    server_tab: ServerTab,

    scroll_index_vertical: ScrollbarState,
    scroll_index_horizontal: ScrollbarState,
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
            *all_tabs.first().unwrap_or(&Self::Hello)
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
            handle,
            tab: TabComponent::new("FlowTabCerts"),
            client_tab_cmp: TabComponent::new("ClientTab"),
            server_tab_cmp: TabComponent::new("ServerTab"),
            root_tab: RootTab::Client,
            client_tab: ClientTab::Hello,
            server_tab: ServerTab::ResolveClientCert,
            scroll_index_vertical: ScrollbarState::default(),
            scroll_index_horizontal: ScrollbarState::default(),
        }
    }

    fn render_client(&mut self, frame: &mut Frame<'_>, area: Rect) {
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
        frame.render_widget(tabs, layout[0]);
        match self.client_tab {
            ClientTab::Hello => self.render_client_hello(frame, layout[1]),
            ClientTab::Certs => self.render_client_cert(frame, layout[1]),
            ClientTab::Tls => self.render_client_tls(frame, layout[1]),
        }
    }

    fn render_client_hello(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let data = process_client_hello(&self.state.borrow().client.hello);
        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(data.len())
            .viewport_content_length(self.area.height as usize);
        let max_line_length = data
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .max()
            .unwrap_or(0);
        self.scroll_index_horizontal = self
            .scroll_index_horizontal
            .content_length(max_line_length)
            .viewport_content_length(self.area.width as usize);
        kv_paragraph(
            &data,
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
    }

    fn render_client_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
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

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(lines.len())
            .viewport_content_length(self.area.height as usize);

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ));
        frame.render_widget(paragraph, area);
    }

    fn render_client_tls(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let client_tls = &self.state.borrow().client.tls;

        let data = process_client_tls(client_tls);

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(data.len())
            .viewport_content_length(self.area.height as usize);

        kv_paragraph(
            &data,
            frame,
            area,
            Some("Tls"),
            self.focus.get(),
            (
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ),
        );
    }

    fn render_server(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        let tab_titles: Vec<Line> = ServerTab::all().iter().map(|v| v.title().into()).collect();

        let tabs = themed_tabs(
            Some("Server"),
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

    fn render_resolve_client_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let certs = &self.state.borrow().server.resolve_client_cert;
        let mut lines = vec![];

        match &certs {
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

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(lines.len())
            .viewport_content_length(self.area.height as usize);

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ));
        frame.render_widget(paragraph, area);
    }

    fn render_server_cert(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let certs = &self.state.borrow().server.certs;
        let mut lines = vec![];
        let header_style = Style::default().bold().underlined();

        match certs {
            Some(capture) => match &capture.cert {
                Some(cert) => {
                    lines.push(Line::from(vec![Span::styled("End entity", header_style)]));
                    match CertInfo::from_der(cert.end_entity.clone()) {
                        Some(cert_info) => {
                            render_cert(&cert_info, &mut lines);
                        }
                        None => {
                            lines.push("Failed to render cert".into());
                        }
                    }
                    for (index, certificate) in cert.intermediates.iter().enumerate() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("Intermediatary {index}"),
                            header_style,
                        )]));
                        match CertInfo::from_der(certificate.clone()) {
                            Some(ci) => {
                                render_cert(&ci, &mut lines);
                            }
                            None => {
                                lines.push("Failed to render cert".into());
                            }
                        }
                    }
                }
                None => {
                    lines.push("No certs".into());
                }
            },
            None => {
                lines.push("No data".into());
            }
        }

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(lines.len())
            .viewport_content_length(self.area.height as usize);

        let paragraph = Paragraph::new(lines)
            .block(themed_block(None, self.focus.get()))
            .wrap(Wrap { trim: false })
            .scroll((
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ));
        frame.render_widget(paragraph, area);

        render_vertical_scrollbar(frame, area, &mut self.scroll_index_vertical);
        render_horizontal_scrollbar(frame, area, &mut self.scroll_index_horizontal);
    }

    fn render_server_tls(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let tls = &self.state.borrow().server.tls;
        let data = process_server_tls(tls);

        self.scroll_index_vertical = self
            .scroll_index_vertical
            .content_length(data.len())
            .viewport_content_length(self.area.height as usize);
        kv_paragraph(
            &data,
            frame,
            area,
            Some("Tls"),
            self.focus.get(),
            (
                self.scroll_index_vertical.get_position() as u16,
                self.scroll_index_horizontal.get_position() as u16,
            ),
        );
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

impl HasFocus for FlowDetailsCerts {
    fn build(&self, builder: &mut FocusBuilder) {
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
    fn handle_action(&mut self, action: Action) -> ActionResult {
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
        if self.focus.get() {
            match action {
                Action::Down => {
                    self.scroll_index_vertical.next();
                    return ActionResult::Consumed;
                }
                Action::Up => {
                    self.scroll_index_vertical.prev();
                    return ActionResult::Consumed;
                }
                Action::Left => {
                    self.scroll_index_horizontal.prev();
                    return ActionResult::Consumed;
                }
                Action::Right => {
                    self.scroll_index_horizontal.next();
                    return ActionResult::Consumed;
                }
                Action::Start => {
                    self.scroll_index_horizontal.first();
                    return ActionResult::Consumed;
                }
                Action::End => {
                    self.scroll_index_horizontal.last();
                    return ActionResult::Consumed;
                }
                Action::PageUp => {
                    for _ in 0..self.area.height as usize {
                        self.scroll_index_vertical.prev();
                    }
                    return ActionResult::Consumed;
                }
                Action::PageDown => {
                    for _ in 0..self.area.height as usize {
                        self.scroll_index_vertical.next();
                    }
                    return ActionResult::Consumed;
                }
                Action::Top => {
                    self.scroll_index_vertical.first();
                    return ActionResult::Consumed;
                }
                Action::Bottom => {
                    self.scroll_index_vertical.last();
                    return ActionResult::Consumed;
                }
                _ => {}
            }
        }
        ActionResult::Ignored
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        let tab_titles: Vec<Line> = RootTab::all().iter().map(|v| v.title().into()).collect();

        let tabs = themed_tabs(
            Some("Certs"),
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

    fn area(&self) -> Rect {
        self.area
    }
}
