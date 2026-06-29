use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::layout::{Constraint, Layout, Rect};
use roxy_shared::cert::{
    CapturedResolveClientCert, ClientTlsConnectionData, ServerVerificationCapture,
};
use strum::EnumIter;

use crate::ui::{
    flow::certs::{
        server_certs::ServerCertsComponent,
        server_resolve_client_cert::ServerResolveClientCertComponent,
        server_tls::process_server_tls,
    },
    framework::{component::Component, kv_component::KvComponent, tab::TabComponent},
};

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
}

#[derive(Default, Clone)]
pub(crate) struct ServerState {
    pub(crate) resolve_client_cert: Option<CapturedResolveClientCert>,
    pub(crate) certs: Option<ServerVerificationCapture>,
    pub(crate) tls: Option<ClientTlsConnectionData>,
}

pub(crate) struct ServerCertificateComponent {
    area: Rect,
    focus: FocusFlag,

    tab: TabComponent,

    resolve_client_component: ServerResolveClientCertComponent,
    cert_component: ServerCertsComponent,
    tls_component: KvComponent,
}

impl ServerCertificateComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ClientCertificateComponent"),
            tab: TabComponent::new(
                "Server".to_string(),
                ServerTab::all()
                    .iter()
                    .map(|t| t.title().to_string())
                    .collect(),
            ),
            resolve_client_component: ServerResolveClientCertComponent::new(),
            cert_component: ServerCertsComponent::new(),
            // tls_component: ServerTlsComponent::new(),
            tls_component: KvComponent::new("Server TLS"),
        }
    }

    pub(crate) fn set_state(&mut self, server_state: &ServerState) {
        self.resolve_client_component
            .set_state(&server_state.resolve_client_cert);
        self.cert_component.set_state(&server_state.certs);
        self.tls_component
            .set_state(process_server_tls(&server_state.tls));
    }
}

impl Component for ServerCertificateComponent {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        if self.tab.current_tab == 0 {
            vec![&mut self.tab, &mut self.resolve_client_component]
        } else if self.tab.current_tab == 1 {
            vec![&mut self.tab, &mut self.cert_component]
        } else {
            vec![&mut self.tab, &mut self.tls_component]
        }
    }
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) -> color_eyre::eyre::Result<()> {
        self.area = area;
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

        self.tab.render(frame, layout[0])?;
        let tab = ServerTab::all()[self.tab.current_tab];
        match tab {
            ServerTab::ResolveClientCert => self.resolve_client_component.render(frame, layout[1]),
            ServerTab::Certs => self.cert_component.render(frame, layout[1]),
            ServerTab::Tls => self.tls_component.render(frame, layout[1]),
        }
    }
}

impl HasFocus for ServerCertificateComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.tab);
        let tab = ServerTab::all()[self.tab.current_tab];
        match tab {
            ServerTab::ResolveClientCert => builder.widget(&self.resolve_client_component),
            ServerTab::Certs => builder.widget(&self.cert_component),
            ServerTab::Tls => builder.widget(&self.tls_component),
        };

        builder.end(tag);
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }
}
