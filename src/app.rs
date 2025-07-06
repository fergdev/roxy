use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use color_eyre::Result;
use crossterm::event::KeyEvent;
use rat_focus::{Focus, FocusBuilder};
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tracing::trace;

use crate::config::ConfigManager;
use crate::event::{Action, Mode};
use crate::flow::FlowStore;
use crate::tui::{Event, Tui};
use crate::ui::framework::component::{ActionResult, Component, KeyEventResult};
use crate::ui::framework::notify::Notifier;
use crate::ui::framework::theme::set_theme;
use crate::ui::home::HomeComponent;
use crate::ui::log::LogLine;

pub const ITEM_HEIGHT: usize = 4;

pub struct App {
    config_manager: ConfigManager,
    home: HomeComponent,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    pub fn new(
        config_manager: ConfigManager,
        flow_store: FlowStore,
        log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
        notifier: Notifier,
    ) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let home = HomeComponent::new(
            config_manager.clone(),
            flow_store.clone(),
            log_buffer.clone(),
            notifier,
        );
        Self {
            config_manager,
            home,
            should_quit: false,
            should_suspend: false,
            mode: Mode::Normal,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?.mouse(true).tick_rate(4.0).frame_rate(60.0);

        tui.enter()?;

        // for component in self.home.iter_mut() {
        //     component.register_action_handler(self.action_tx.clone())?;
        // }
        // for component in self.home.iter_mut() {
        //     component.init(tui.size()?)?;
        // }

        let action_tx = self.action_tx.clone();
        loop {
            let mut focus = FocusBuilder::build_for(&self.home);
            focus.enable_log();

            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui, &mut focus)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        // for component in self.home.iter_mut() {
        //     if let Some(action) = component.handle_events(event.clone())? {
        //         action_tx.send(action)?;
        //     }
        // }
        if let Some(action) = self.home.handle_events(event.clone())? {
            action_tx.send(action)?;
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        trace!("Key event: {key:?}");

        let action_tx = self.action_tx.clone();
        // let components consume
        // for component in self.home.iter_mut() {
        //     match component.handle_key_event(&key) {
        //         KeyEventResult::Consumed => {
        //             return Ok(());
        //         }
        //         KeyEventResult::Ignored => {}
        //         KeyEventResult::Action(action) => {
        //             action_tx.send(action)?;
        //         }
        //     }
        // }

        match self.home.handle_key_event(&key) {
            KeyEventResult::Consumed => {
                return Ok(());
            }
            KeyEventResult::Ignored => {}
            KeyEventResult::Action(action) => {
                action_tx.send(action)?;
            }
        }

        let cfg = self.config_manager.rx.borrow();
        let Some(keymap) = cfg.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                trace!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    trace!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui, focus: &mut Focus) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                trace!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::FocusNext => {
                    focus.next();
                }
                Action::FocusPrev => {
                    focus.prev();
                }
                _ => {}
            }
            // for component in self.home.iter_mut() {
            //     if let ActionResult::Action(action) = component.update(action.clone()) {
            //         self.action_tx.send(action)?
            //     };
            // }
            if let ActionResult::Action(action) = self.home.update(action.clone()) {
                self.action_tx.send(action)?
            };
        }
        Ok(())
    }

    // fn print_focus(focus: &Focus) {
    //     if focus.is_log_enabled() {
    //         let focused = focus.focused();
    //         if let Some(focused) = focused {
    //             trace!("Focused component: {}", focused.name());
    //         } else {
    //             trace!("No component is focused");
    //         }
    //     }
    // }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        // TODO: should not clone here
        let theme = self.config_manager.rx.borrow_and_update().theme.clone();
        set_theme(theme.clone());
        tui.draw(|frame| {
            if let Err(error) = self.home.render(frame, frame.area()) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", error)));
            }
        })?;
        Ok(())
    }
}
