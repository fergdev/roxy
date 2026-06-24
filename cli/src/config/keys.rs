use std::collections::HashMap;

use cow_utils::CowUtils;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_deref::{Deref, DerefMut};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use tracing::error;

use crate::{
    event::{Action, Mode},
    notify_error,
};

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

pub fn format_key_sequence(seq: &[KeyEvent]) -> String {
    seq.iter()
        .map(key_event_to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

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
                inner.insert(format_key_sequence(key_seq), action);
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
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map = inner_map
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
        error!("Error parsing {raw}");
        return Err(format!("Unable to parse `{raw}`"));
    }
    let raw = if !raw.contains("><") {
        raw.strip_prefix('<').unwrap_or(raw)
    } else {
        raw
    };
    let sequences = raw
        .split("><")
        .map(|seq| {
            if let Some(s) = seq.strip_prefix('<') {
                s
            } else if let Some(s) = seq.strip_suffix('>') {
                s
            } else {
                seq
            }
        })
        .collect::<Vec<_>>();

    sequences.into_iter().map(parse_key_event).collect()
}

pub fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.cow_to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break,
        };
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "<Esc>" => KeyCode::Esc,
        "<Enter>" => KeyCode::Enter,
        "<Left>" => KeyCode::Left,
        "<Right>" => KeyCode::Right,
        "<Up>" => KeyCode::Up,
        "<Down>" => KeyCode::Down,
        "<Home>" => KeyCode::Home,
        "<End>" => KeyCode::End,
        "<Pageup>" => KeyCode::PageUp,
        "<Pagedown>" => KeyCode::PageDown,
        "<S-Tab>" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "<BS>" => KeyCode::Backspace,
        "<Del>" => KeyCode::Delete,
        "<Insert>" => KeyCode::Insert,
        "<F1>" => KeyCode::F(1),
        "<F2>" => KeyCode::F(2),
        "<F3>" => KeyCode::F(3),
        "<F4>" => KeyCode::F(4),
        "<F5>" => KeyCode::F(5),
        "<F6>" => KeyCode::F(6),
        "<F7>" => KeyCode::F(7),
        "<F8>" => KeyCode::F(8),
        "<F9>" => KeyCode::F(9),
        "<F10>" => KeyCode::F(10),
        "<F11>" => KeyCode::F(11),
        "<F12>" => KeyCode::F(12),
        "<Space>" => KeyCode::Char(' '),
        "<kMinus>" => KeyCode::Char('-'),
        "<Tab>" => KeyCode::Tab,
        c if c.starts_with('<') && c.ends_with('>') => {
            if c.starts_with("<C-") {
                c.strip_prefix("<C-")
                    .and_then(|s| s.strip_suffix('>'))
                    .and_then(|s| s.chars().next())
                    .map(|c| {
                        modifiers.insert(KeyModifiers::CONTROL);
                        KeyCode::Char(c)
                    })
                    .ok_or_else(|| format!("Unable to parse {raw}"))?
            } else {
                return Err(format!("Unable to parse {raw}"));
            }
        }
        c if c.len() == 1 => {
            if let Some(mut c) = c.chars().next() {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    c = c.to_ascii_uppercase();
                }
                KeyCode::Char(c)
            } else {
                return Err(format!("Unable to parse {raw}"));
            }
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "<BS>",
        KeyCode::Enter => "<Enter>",
        KeyCode::Left => "<Left>",
        KeyCode::Right => "<Right>",
        KeyCode::Up => "<Up>",
        KeyCode::Down => "<Down>",
        KeyCode::Home => "<Home>",
        KeyCode::End => "<End>",
        KeyCode::PageUp => "<Pageup>",
        KeyCode::PageDown => "<Pagedown>",
        KeyCode::Tab => "<Tab>",
        KeyCode::BackTab => "<S-Tab>",
        KeyCode::Delete => "<Delete>",
        KeyCode::Insert => "<Insert>",
        KeyCode::F(c) => {
            char = format!("<F{c}>");
            &char
        }
        KeyCode::Char(' ') => "<Space>",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "<Esc>",
        _ => {
            todo!("Unsupported key code: {:?}", key_event.code);
        }
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    pub fn key_event_to_string_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "x");
    }

    #[test]
    pub fn key_event_to_string_shift_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "x");
    }

    #[test]
    pub fn key_event_to_string_ctl_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-x>");
    }

    #[test]
    pub fn key_event_to_string_ctl_shift_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('X'), KeyModifiers::CONTROL);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-X>");
    }

    #[test]
    pub fn key_event_to_string_alt_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<M-x>");
    }

    #[test]
    pub fn key_event_to_string_alt_shift_x() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Char('X'), KeyModifiers::ALT);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<M-X>");
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_x() {
        let key_event = crossterm::event::KeyEvent::new(
            KeyCode::Char('X'),
            KeyModifiers::ALT | KeyModifiers::CONTROL,
        );
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-M-x>");
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_shift_x() {
        let key_event = crossterm::event::KeyEvent::new(
            KeyCode::Char('X'),
            KeyModifiers::ALT | KeyModifiers::CONTROL,
        );
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-M-X>");
    }

    #[test]
    pub fn key_event_to_string_esc() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<Esc>");
    }

    #[test]
    pub fn key_event_to_string_ctl_esc() {
        let key_event = crossterm::event::KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL);
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-Esc>");
    }

    #[test]
    pub fn key_event_to_string_ctl_alt_esc() {
        let key_event = crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::ALT | KeyModifiers::CONTROL,
        );
        let key_str = super::key_event_to_string(&key_event);
        assert_eq!(key_str, "<C-M-Esc>");
    }
}
