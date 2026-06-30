use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, EnumString)]
pub enum Action {
    Quit,
    Back,
    ClearScreen,
    Error(String),

    Up,
    Down,
    Left,
    Right,

    PageUp,
    PageDown,

    Top,
    Bottom,
    Start,
    End,

    // Views
    EditConfig,
    LogView,
    FpsView,
    Connections,

    // Focus
    FocusNext,
    FocusPrev,
    FocusReq(usize), // (widget_id)

    Select,
    Add,
    Delete,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyInputMode {
    #[default]
    Normal,
    Insert,
}
