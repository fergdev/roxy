use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, EnumString)]
pub enum Action {
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
    FocusReq(usize), // (widget_id)

    Top,
    Bottom,
    // TODO: add these actions
    Start,
    End,

    EditConfig,
    LogView,
    FpsView,

    Add,
    Delete,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyInputMode {
    #[default]
    Normal,
    Insert,
}
