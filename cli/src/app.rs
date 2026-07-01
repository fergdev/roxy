use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use rat_focus::{Focus, FocusBuilder};
use ratatui::layout::Rect;
use roxy_proxy::flow_store::FlowStore;
use roxy_proxy::proxy::ProxyManager;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::config::manager::ConfigManager;
use crate::key_handler::KeyHandler;
use crate::tui::{Tui, TuiEvent};
use crate::ui::framework::component::{Component, DispatchCancellation};
use crate::ui::framework::notify::NotifierComponent;
use crate::ui::framework::theme::set_theme;
use crate::ui::home::HomeComponent;
use crate::ui::log::LogLine;

pub const ITEM_HEIGHT: usize = 4;

pub struct App {
    _proxy_manager: ProxyManager,
    config_manager: ConfigManager,
    home: HomeComponent,
    should_quit: bool,
    should_suspend: bool,
    key_handler: KeyHandler,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    pub fn new(
        proxy_manager: ProxyManager,
        config_manager: ConfigManager,
        flow_store: FlowStore,
        log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
        notifier: NotifierComponent,
    ) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let home = HomeComponent::new(
            config_manager.clone(),
            flow_store.clone(),
            log_buffer.clone(),
            notifier,
            action_tx.clone(),
        );
        let key_handler = KeyHandler::new(config_manager.clone(), action_tx.clone());
        Self {
            _proxy_manager: proxy_manager,
            config_manager,
            home,
            should_quit: false,
            should_suspend: false,
            key_handler,
            action_tx,
            action_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut tui = Tui::new()?.mouse(true).tick_rate(4.0).frame_rate(60.0);
        tui.enter()?;
        loop {
            let mut focus = FocusBuilder::build_for(&self.home);

            // Enable for logging of focus related changes
            // focus.enable_log();

            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui, &mut focus)?;

            if self.should_suspend {
                tui.suspend()?;
                tui.terminal.clear()?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<(), Box<dyn std::error::Error>> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            TuiEvent::Quit => self.should_quit = true,
            TuiEvent::Render => self.render(tui)?,
            TuiEvent::Resize(w, h) => {
                self.handle_resize(tui, w, h)?;
            }
            TuiEvent::Key(key) => self.key_handler.handle_key_event(key)?,
            _ => {}
        }
        if let Err(DispatchCancellation::Bubble(action)) = self.home.dispatch_tui_events(&event) {
            action_tx.send(action)?;
        }
        Ok(())
    }

    fn handle_actions(
        &mut self,
        tui: &mut Tui,
        focus: &mut Focus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                Action::Quit => self.should_quit = true,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::FocusNext => {
                    focus.next();
                }
                Action::FocusPrev => {
                    focus.prev();
                }
                Action::FocusReq(widget_id) => {
                    focus.by_widget_id(widget_id);
                }
                _ => {}
            }
            if let Err(DispatchCancellation::Bubble(action)) = self.home.dispatch_action(&action) {
                self.action_tx.send(action)?
            };
        }
        Ok(())
    }

    fn handle_resize(
        &mut self,
        tui: &mut Tui,
        w: u16,
        h: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<(), Box<dyn std::error::Error>> {
        let theme = self.config_manager.rx.borrow_and_update().theme.clone();
        set_theme(theme);
        tui.draw(|frame| {
            self.home.render(frame, frame.area());
        })?;
        Ok(())
    }
}
