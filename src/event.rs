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

    FocusNext,
    FocusPrev,

    Top,
    Bottom,

    EditConfig,
    LogView,
    FpsView,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}
