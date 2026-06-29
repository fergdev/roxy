#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ConfigTab {
    App,
    Proxy,
    KeyBinds,
    Theme,
}

impl ConfigTab {
    pub(crate) fn all() -> &'static [ConfigTab] {
        &[Self::App, Self::Proxy, Self::KeyBinds, Self::Theme]
    }

    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::App => "App",
            Self::Proxy => "Proxy",
            Self::KeyBinds => "Keys",
            Self::Theme => "Theme",
        }
    }
}
