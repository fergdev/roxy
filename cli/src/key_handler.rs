use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use crossterm::event::KeyEvent;
use tokio::{
    sync::{Mutex, mpsc::UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tracing::error;

use crate::{
    action::{Action, KeyInputMode},
    config::manager::ConfigManager,
    key_trie::KeyTrie,
};

/// The timeout for key input combinations to fire if there are other potential
/// combinations incoming. Exampe: gg has been input, but ggg is mapped too.
///
/// 300 is the len for the my vim config, which feels comfy
const TIMEOUT_LEN: u64 = 300;

#[derive(Clone)]
pub struct KeyHandler {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    action_tx: UnboundedSender<Action>,

    mode: KeyInputMode,
    key_trie: KeyTrie,

    last_tick_key_events: Vec<KeyEvent>,
    last_key_tick: Option<SystemTime>,

    /// Handle for when timeout when waiting for another key event.
    more_action_timeout_handle: Option<JoinHandle<()>>,
}

/// Map's keys to actions based on the current configuation.
impl KeyHandler {
    pub fn new(config_manager: ConfigManager, action_tx: UnboundedSender<Action>) -> Self {
        let mut config_rx = config_manager.rx.clone();
        config_rx.mark_changed();

        let inner = Arc::new(Mutex::new(Inner {
            mode: KeyInputMode::Normal,
            key_trie: KeyTrie::default(),

            last_tick_key_events: vec![],
            last_key_tick: None,
            more_action_timeout_handle: None,
            action_tx,
        }));

        let shared_inner = inner.clone();

        tokio::spawn(async move {
            loop {
                match config_rx.changed().await {
                    Ok(_) => {
                        let mut guard = shared_inner.lock().await;
                        guard.key_trie.clear();
                        let cfg = config_rx.borrow_and_update();
                        if let Some(bind) = cfg.keybindings.get(&guard.mode) {
                            bind.iter().for_each(|(k, v)| {
                                guard.key_trie.push(k, v.to_owned());
                            });
                        }
                    }
                    Err(err) => {
                        error!("Error watching config `{err}`");
                        break;
                    }
                }
            }
        });

        Self { inner }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        let mut inner = self.inner.try_lock()?;

        if let Some(handle) = &inner.more_action_timeout_handle.take() {
            handle.abort();
        }

        inner.last_tick_key_events.push(key_event);
        let (action, has_more) = inner.key_trie.get(&inner.last_tick_key_events);

        if let Some(action) = action {
            // We have long key combinations available so we will delay sending a final action
            // until all key strokes have been consumed
            if has_more {
                let self_timeout = self.clone();
                let handle = tokio::spawn(async move {
                    let _ = sleep(Duration::from_millis(TIMEOUT_LEN)).await;
                    if let Ok(mut inner) = self_timeout.inner.try_lock() {
                        let _ = inner.action_tx.send(action);
                        inner.last_tick_key_events.drain(..);
                        inner.last_key_tick = None;
                    }
                });
                inner.more_action_timeout_handle.replace(handle);

                return Ok(());
            }

            inner.action_tx.send(action)?;
        }

        // No action or more actions available, clean up
        inner.last_tick_key_events.drain(..);
        inner.last_key_tick = None;

        Ok(())
    }
}
