use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};
use roxy_shared::cert::{CapturedClientHello, ClientVerificationCapture, ServerTlsConnectionData};
use strum::EnumIter;

use crate::ui::{
    flow::certs::{
        client_certs::ClientCertComponent, client_hello::process_client_hello,
        client_tls::process_client_tls,
    },
    framework::{component::Component, kv_component::KvComponent, tab::TabComponent},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub(crate) enum ClientTab {
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
}

#[derive(Default, Clone)]
pub(crate) struct ClientState {
    pub(crate) hello: Option<CapturedClientHello>,
    pub(crate) certs: Option<ClientVerificationCapture>,
    pub(crate) tls: Option<ServerTlsConnectionData>,
}

pub(crate) struct ClientCertificateComponent {
    area: Rect,
    focus: FocusFlag,

    tab: TabComponent,

    hello_component: KvComponent,
    cert_component: ClientCertComponent,
    tls_component: KvComponent,
}

impl ClientCertificateComponent {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ClientCertificateComponent"),
            tab: TabComponent::new(
                "Client".to_string(),
                ClientTab::all()
                    .iter()
                    .map(|t| t.title().to_string())
                    .collect(),
            ),
            hello_component: KvComponent::new("Hello"),
            cert_component: ClientCertComponent::new(),
            tls_component: KvComponent::new("tls"),
        }
    }

    pub(crate) fn set_state(&mut self, client_state: &ClientState) {
        self.hello_component
            .set_state(process_client_hello(&client_state.hello));
        self.cert_component.set_state(&client_state.certs);
        self.tls_component
            .set_state(process_client_tls(&client_state.tls));
    }
}

impl Component for ClientCertificateComponent {
    fn children(&mut self) -> Vec<&mut dyn Component> {
        if self.tab.current_tab == 0 {
            vec![&mut self.tab, &mut self.hello_component]
        } else if self.tab.current_tab == 1 {
            vec![&mut self.tab, &mut self.cert_component]
        } else {
            vec![&mut self.tab, &mut self.tls_component]
        }
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        self.area = area;

        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
        self.tab.render(frame, layout[0])?;

        let tab = ClientTab::all()[self.tab.current_tab];
        match tab {
            ClientTab::Hello => self.hello_component.render(frame, layout[1])?,
            ClientTab::Certs => self.cert_component.render(frame, layout[1])?,
            ClientTab::Tls => self.tls_component.render(frame, layout[1])?,
        }
        Ok(())
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
    fn area(&self) -> Rect {
        self.area
    }
}

impl HasFocus for ClientCertificateComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        let tag = builder.start(self);
        builder.widget(&self.tab);
        let client_tab = ClientTab::all()[self.tab.current_tab];
        match client_tab {
            ClientTab::Hello => builder.widget(&self.hello_component),
            ClientTab::Certs => builder.widget(&self.cert_component),
            ClientTab::Tls => builder.widget(&self.tls_component),
        };
        builder.end(tag)
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}
