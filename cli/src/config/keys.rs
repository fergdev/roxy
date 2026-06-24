use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_deref::{Deref, DerefMut};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};

use crate::{
    event::{Action, KeyInputMode},
    notify_error,
};

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct KeyBindings(pub HashMap<KeyInputMode, HashMap<Vec<KeyEvent>, Action>>);

impl Serialize for KeyBindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (mode, bindings) in &self.0 {
            let mode_key = format!("{mode:?}");
            let mut inner = HashMap::new();

            for (key_seq, action) in bindings {
                let keys = key_seq
                    .iter()
                    .map(key_event_to_string)
                    .collect::<Vec<_>>()
                    .join("");
                inner.insert(keys, action);
            }

            map.serialize_entry(&mode_key, &inner)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let parsed_map =
            HashMap::<KeyInputMode, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, key_sequence_to_actions)| {
                let converted_inner_map = key_sequence_to_actions
                    .into_iter()
                    .filter_map(|(key_str, cmd)| match parse_key_sequence(&key_str) {
                        Ok(seq) => Some((seq, cmd)),
                        Err(e) => {
                            notify_error!("Failed to parse key '{}': {}", key_str, e);
                            None
                        }
                    })
                    .collect();
                (mode, converted_inner_map)
            })
            .collect();

        Ok(KeyBindings(keybindings))
    }
}

pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!(
            "Unable to parse `{raw}` inconsistent opening and closing <>"
        ));
    }
    let sequences = raw.split_inclusive(">").collect::<Vec<_>>();

    let sequences = sequences
        .iter()
        .flat_map(|f| {
            if f.contains(">") {
                vec![f.to_string()]
            } else {
                f.chars()
                    .map(|c| format!("{c}").to_string())
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();

    sequences.into_iter().map(|s| parse_key_code(&s)).collect()
}

fn parse_key_code(raw: &str) -> Result<KeyEvent, String> {
    let (mut modifiers, raw) = if raw.starts_with("<C-M-") {
        (
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            raw.trim_start_matches("<C-M-").trim_end_matches(">"),
        )
    } else if raw.starts_with("<C-") {
        (
            KeyModifiers::CONTROL,
            raw.trim_start_matches("<C-").trim_end_matches(">"),
        )
    } else if raw.starts_with("<M-") {
        (
            KeyModifiers::ALT,
            raw.trim_start_matches("<M-").trim_end_matches(">"),
        )
    } else {
        (
            KeyModifiers::NONE,
            raw.trim_start_matches("<").trim_end_matches(">"),
        )
    };

    let key_code = match raw {
        "Esc" => KeyCode::Esc,
        "Enter" => KeyCode::Enter,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Pageup" => KeyCode::PageUp,
        "Pagedown" => KeyCode::PageDown,
        "S-Tab" => {
            modifiers |= KeyModifiers::SHIFT;
            KeyCode::BackTab
        }
        "BS" => KeyCode::Backspace,
        "Del" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "F1" => KeyCode::F(1),
        "F2" => KeyCode::F(2),
        "F3" => KeyCode::F(3),
        "F4" => KeyCode::F(4),
        "F5" => KeyCode::F(5),
        "F6" => KeyCode::F(6),
        "F7" => KeyCode::F(7),
        "F8" => KeyCode::F(8),
        "F9" => KeyCode::F(9),
        "F10" => KeyCode::F(10),
        "F11" => KeyCode::F(11),
        "F12" => KeyCode::F(12),
        "Space" => KeyCode::Char(' '),
        "kMinus" => KeyCode::Char('-'),
        "Tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            if let Some(c) = c.chars().next() {
                if c.is_uppercase() {
                    modifiers |= KeyModifiers::SHIFT;
                }
                KeyCode::Char(c)
            } else {
                return Err(format!("Unable to parse {raw}"));
            }
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };

    Ok(KeyEvent::new(key_code, modifiers))
}

fn key_event_to_string(key_event: &KeyEvent) -> String {
    let mut modifiers = Vec::with_capacity(3);
    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("C");
    }
    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("M");
    }
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "BS",
        KeyCode::Enter => "Enter",
        KeyCode::Left => "Left",
        KeyCode::Right => "Right",
        KeyCode::Up => "Up",
        KeyCode::Down => "Down",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "Pageup",
        KeyCode::PageDown => "Pagedown",
        KeyCode::Tab => "Tab",
        KeyCode::BackTab => "S-Tab",
        KeyCode::Delete => "Delete",
        KeyCode::Insert => "Insert",
        KeyCode::F(c) => {
            char = format!("F{c}");
            &char
        }
        KeyCode::Char(' ') => "Space",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "Esc",
        _ => {
            todo!("Unsupported key code: {:?}", key_event.code);
        }
    };

    if key_code.len() > 1 || !modifiers.is_empty() {
        if modifiers.is_empty() {
            format!("<{key_code}>")
        } else {
            let modifiers = modifiers.join("-");
            format!("<{modifiers}-{key_code}>")
        }
    } else {
        key_code.to_string()
    }
}

pub fn key_sequence_to_string(seq: &[KeyEvent]) -> String {
    let debug_string = seq
        .iter()
        .map(key_event_to_string)
        .collect::<Vec<String>>()
        .join("");
    debug_string.to_string()
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    pub fn parse_key_sequence_cx_cx() {
        assert_eq!(
            vec![
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            ],
            super::parse_key_sequence("<C-x><C-x>").unwrap()
        );
    }

    #[test]
    pub fn parse_key_sequence_g_g() {
        assert_eq!(
            vec![
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            ],
            super::parse_key_sequence("gg").unwrap()
        );
    }

    #[test]
    pub fn parse_key_sequence_cx_g() {
        assert_eq!(
            vec![
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            ],
            super::parse_key_sequence("<C-x>g").unwrap()
        );
    }

    #[test]
    pub fn parse_key_code_x() {
        assert_eq!(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            super::parse_key_code("x").unwrap(),
        );
        assert_eq!(
            KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE),
            super::parse_key_code("X").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_ctrl_x() {
        assert_eq!(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            super::parse_key_code("<C-x>").unwrap(),
        );
        assert_eq!(
            KeyEvent::new(KeyCode::Char('X'), KeyModifiers::CONTROL),
            super::parse_key_code("<C-X>").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_ctrl_alt_x() {
        assert_eq!(
            KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ),
            super::parse_key_code("<C-M-x>").unwrap(),
        );
        assert_eq!(
            KeyEvent::new(
                KeyCode::Char('X'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ),
            super::parse_key_code("<C-M-X>").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_esc() {
        assert_eq!(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            super::parse_key_code("<Esc>").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_ctrl_esc() {
        assert_eq!(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL),
            super::parse_key_code("<C-Esc>").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_ctrl_alt_esc() {
        assert_eq!(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL | KeyModifiers::ALT),
            super::parse_key_code("<C-M-Esc>").unwrap(),
        );
    }

    #[test]
    pub fn parse_key_code_s_tab() {
        assert_eq!(
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            super::parse_key_code("<S-Tab>").unwrap(),
        );
    }

    #[test]
    pub fn key_event_to_string_x() {
        assert_eq!(
            "x",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        );
    }

    #[test]
    pub fn key_event_to_string_shift_x() {
        assert_eq!(
            "X",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_x() {
        assert_eq!(
            "<C-x>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_shift_x() {
        assert_eq!(
            "<C-X>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('X'), KeyModifiers::CONTROL))
        );
    }

    #[test]
    pub fn key_event_to_string_alt_x() {
        assert_eq!(
            "<M-x>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT))
        );
    }

    #[test]
    pub fn key_event_to_string_alt_shift_x() {
        assert_eq!(
            "<M-X>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Char('X'), KeyModifiers::ALT))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_x() {
        assert_eq!(
            "<C-M-x>",
            super::key_event_to_string(&KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::ALT | KeyModifiers::CONTROL
            ))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_shift_x() {
        assert_eq!(
            "<C-M-X>",
            super::key_event_to_string(&KeyEvent::new(
                KeyCode::Char('X'),
                KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT
            ))
        );
    }

    #[test]
    pub fn key_event_to_string_esc() {
        assert_eq!(
            "<Esc>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_esc() {
        assert_eq!(
            "<C-Esc>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL))
        );
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_esc() {
        assert_eq!(
            "<C-M-Esc>",
            super::key_event_to_string(&KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::ALT | KeyModifiers::CONTROL
            ))
        );
    }

    #[test]
    pub fn key_event_to_string_s_tab() {
        assert_eq!(
            "<S-Tab>",
            super::key_event_to_string(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT))
        );
    }
}
