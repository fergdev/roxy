use std::collections::HashMap;

use crossterm::event::KeyEvent;
use tracing::error;

use crate::action::Action;

/// Simple key_trie to make keybindings to actions while determining if more keys are expected
/// from the user. This is used to determine if a keybinding is complete or if more keys are
/// expected.
#[derive(Default)]
pub struct KeyTrie {
    root: KeyNode,
}

impl KeyTrie {
    pub fn push(&mut self, events: &[KeyEvent], action: Action) {
        self.root.push(events, action);
    }

    pub fn get(&self, events: &[KeyEvent]) -> (Option<Action>, bool) {
        self.root.get(events)
    }

    pub fn clear(&mut self) {
        self.root.clear()
    }
}

#[derive(Default)]
pub struct KeyNode {
    nodes: HashMap<KeyEvent, KeyNode>,
    action: Option<Action>,
}

impl KeyNode {
    pub fn push(&mut self, events: &[KeyEvent], action: Action) {
        if events.is_empty() {
            if self.action.is_none() {
                self.action.replace(action);
            } else {
                let sa = &self.action;
                error!("Can't insert action {action} already set {sa:?}");
            }
            return;
        }
        let ev = match events.first() {
            Some(event) => event,
            None => {
                error!("Can't push");
                return;
            }
        };
        let node = self.nodes.get_mut(ev);
        if let Some(node) = node {
            node.push(&events[1..], action);
        } else {
            let mut node = KeyNode::default();
            node.push(&events[1..], action);
            self.nodes.insert(*ev, node);
        }
    }

    pub fn get(&self, events: &[KeyEvent]) -> (Option<Action>, bool) {
        if let Some(first) = events.first() {
            if let Some(node) = self.nodes.get(first) {
                node.get(&events[1..])
            } else {
                (None, false)
            }
        } else {
            (self.action.clone(), !self.nodes.is_empty())
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

#[cfg(test)]
pub mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::action::Action;

    use super::KeyTrie;

    #[test]
    pub fn key_trie_single() {
        let mut key_trie = KeyTrie::default();

        let events = &[KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)];
        key_trie.push(events, Action::Back);

        let (action, more) = key_trie.get(events);
        assert_eq!(Some(Action::Back), action);
        assert!(!more);
    }

    #[test]
    pub fn key_trie_nested() {
        let mut key_trie = KeyTrie::default();

        let events = &[
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ];
        key_trie.push(events, Action::Back);

        let (action, more) = key_trie.get(events);
        assert_eq!(Some(Action::Back), action);
        assert!(!more);
    }

    #[test]
    pub fn key_trie_multple_actions() {
        let mut key_trie = KeyTrie::default();

        let events = &[
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ];
        key_trie.push(events, Action::Back);
        key_trie.push(&events[1..], Action::Up);

        let (action, more) = key_trie.get(&events[1..]);
        assert_eq!(Some(Action::Up), action);
        assert!(more);
    }

    #[test]
    pub fn key_trie_clear_empties_nodes() {
        let mut key_trie = KeyTrie::default();

        let events = &[
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ];
        key_trie.push(events, Action::Back);
        key_trie.push(&events[1..], Action::Up);

        key_trie.clear();

        let (action, more) = key_trie.get(&events[1..]);
        assert_eq!(None, action);
        assert!(!more);
    }
}
