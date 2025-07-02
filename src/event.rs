use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, EnumString)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    Back,
    ClearScreen,
    Error(String),

    Select,

    Up,
    Down,
    Left,
    Right,

    Top,
    Bottom,

    EditConfig,
    LogView,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppEvent {
    #[default]
    Quit,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}
