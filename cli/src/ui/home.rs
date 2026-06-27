use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{config::manager::ConfigManager, event::Action, tui::TuiEvent};

use super::{
    config::ConfigEditor,
    flow::{details::FlowDetails, list::FlowList},
    fps_counter::FpsCounter,
    framework::{
        component::{ActionResult, Component, KeyEventResult},
        notify::Notifier,
    },
    log::{LogLine, LogViewer},
    quit_popup::QuitPopup,
    splash::Splash,
};

use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{Frame, layout::Rect};
use roxy_proxy::flow::FlowStore;

pub struct HomeComponent {
    focus: FocusFlag,
    flow_store: FlowStore,
    active_view: ActiveView,
    active_popup: Option<ActivePopup>,
    splash: Splash,
    flow_list: FlowList,
    flow_details: FlowDetails,
    config_editor: ConfigEditor,
    quit_popup: QuitPopup,
    log_viewer: LogViewer,
    fps_counter: FpsCounter,
    notifier: Notifier,
    config_manager: ConfigManager,
}

impl HomeComponent {
    pub fn new(
        config_manager: ConfigManager,
        flow_store: FlowStore,
        log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
        notifier: Notifier,
    ) -> Self {
        let port = config_manager.rx.borrow().app.proxy.port;
        let splash = Splash::new(port);
        let flow_list = FlowList::new(flow_store.clone());
        Self {
            focus: FocusFlag::new().with_name("Home"),
            flow_store: flow_store.clone(),
            active_view: ActiveView::Splash,
            active_popup: None,
            splash,
            flow_list,
            config_editor: ConfigEditor::new(config_manager.clone()),
            quit_popup: QuitPopup::default(),
            flow_details: FlowDetails::new(flow_store.clone()),
            log_viewer: LogViewer::new(log_buffer),
            fps_counter: FpsCounter::new(),
            notifier,
            config_manager,
        }
    }
}

impl HasFocus for HomeComponent {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        let tag = builder.start(self);

        if let Some(active_popup) = self.active_popup {
            match active_popup {
                ActivePopup::ConfigEditor => {
                    builder.widget(&self.config_editor);
                }
                ActivePopup::QuitPopup => {
                    builder.widget(&self.quit_popup);
                }
                ActivePopup::FlowDetails => {
                    builder.widget(&self.flow_details);
                }
                ActivePopup::LogViewer => {
                    builder.widget(&self.log_viewer);
                }
            };
        } else {
            match self.active_view {
                ActiveView::Splash => {
                    builder.widget(&self.splash);
                }
                ActiveView::FlowList => {
                    builder.widget(&self.flow_list);
                }
            }
        }

        builder.end(tag);
    }

    fn area(&self) -> Rect {
        Rect::default()
    }

    fn focus(&self) -> rat_focus::FocusFlag {
        self.focus.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Splash,
    FlowList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePopup {
    ConfigEditor,
    QuitPopup,
    FlowDetails,
    LogViewer,
}

impl Component for HomeComponent {
    fn handle_events(&mut self, event: TuiEvent) -> Result<Option<Action>> {
        let action = match event {
            TuiEvent::Tick => {
                if self.flow_store.flows.is_empty() {
                    self.active_view = ActiveView::Splash;
                } else {
                    self.active_view = ActiveView::FlowList;
                }
                None
            }
            _ => None,
        };
        Ok(action)
    }

    fn update(&mut self, action: Action) -> ActionResult {
        let _ = self.fps_counter.update(action.clone());

        let res = match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.update(action.clone()),
            Some(ActivePopup::QuitPopup) => self.quit_popup.update(action.clone()),
            Some(ActivePopup::FlowDetails) => self.flow_details.update(action.clone()),
            Some(ActivePopup::LogViewer) => self.log_viewer.update(action.clone()),
            None => ActionResult::Ignored,
        };

        if res != ActionResult::Ignored {
            return res;
        }

        let res = match self.active_view {
            ActiveView::Splash => self.splash.update(action.clone()),
            ActiveView::FlowList => self.flow_list.update(action.clone()),
        };

        if res != ActionResult::Ignored {
            return res;
        }

        match action {
            Action::LogView => {
                self.active_popup = Some(ActivePopup::LogViewer);
                ActionResult::Consumed
            }
            Action::EditConfig => {
                self.active_popup = Some(ActivePopup::ConfigEditor);
                self.config_editor.shown();
                ActionResult::Consumed
            }
            Action::Back => match self.active_popup {
                Some(_) => {
                    self.active_popup = None;
                    ActionResult::Consumed
                }
                _ => {
                    if !self.config_manager.rx.borrow().app.confirm_quit {
                        ActionResult::Action(Action::Quit)
                    } else {
                        self.active_popup = Some(ActivePopup::QuitPopup);
                        self.quit_popup.reset();
                        ActionResult::Consumed
                    }
                }
            },
            Action::Select => {
                if let Some(id) = self.flow_list.selected_id() {
                    self.flow_details.set_flow(id);
                    self.active_popup = Some(ActivePopup::FlowDetails);
                    ActionResult::Consumed
                } else {
                    ActionResult::Ignored
                }
            }

            _ => ActionResult::Ignored,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        match self.active_view {
            ActiveView::Splash => self.splash.render(f, area)?,
            ActiveView::FlowList => self.flow_list.render(f, area)?,
        };

        self.fps_counter.render(f, area)?;
        match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.render(f, area)?,
            Some(ActivePopup::QuitPopup) => self.quit_popup.render(f, area)?,
            Some(ActivePopup::FlowDetails) => self.flow_details.render(f, area)?,
            Some(ActivePopup::LogViewer) => self.log_viewer.render(f, area)?,
            None => {}
        };

        self.notifier.render(f, area);
        Ok(())
    }

    fn handle_key_event(&mut self, key: &crossterm::event::KeyEvent) -> KeyEventResult {
        let res = match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.handle_key_event(key),
            Some(ActivePopup::QuitPopup) => self.quit_popup.handle_key_event(key),
            Some(ActivePopup::FlowDetails) => self.flow_details.handle_key_event(key),
            Some(ActivePopup::LogViewer) => self.log_viewer.handle_key_event(key),
            _ => KeyEventResult::Ignored,
        };

        match res {
            KeyEventResult::Consumed => return res,
            KeyEventResult::Action(_) => return res,
            _ => {}
        };
        let res = match self.active_view {
            ActiveView::Splash => self.splash.handle_key_event(key),
            ActiveView::FlowList => self.flow_list.handle_key_event(key),
        };
        match res {
            KeyEventResult::Consumed => return res,
            KeyEventResult::Action(_) => return res,
            _ => {}
        };
        KeyEventResult::Ignored
    }

    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Result<Option<Action>> {
        let res = match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.handle_mouse_event(mouse)?,
            Some(ActivePopup::QuitPopup) => self.quit_popup.handle_mouse_event(mouse)?,
            Some(ActivePopup::FlowDetails) => self.flow_details.handle_mouse_event(mouse)?,
            Some(ActivePopup::LogViewer) => self.log_viewer.handle_mouse_event(mouse)?,
            None => None,
        };

        if res.is_some() {
            return Ok(res);
        }

        let res = match self.active_view {
            ActiveView::Splash => self.splash.handle_mouse_event(mouse)?,
            ActiveView::FlowList => self.flow_list.handle_mouse_event(mouse)?,
        };

        Ok(res)
    }
}
