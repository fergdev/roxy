use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{action::Action, config::manager::ConfigManager, tui::TuiEvent};

use super::{
    config::ConfigEditor,
    flow::{details::FlowDetails, list::FlowList},
    fps_counter::FpsCounter,
    framework::{
        component::{ActionResult, Component},
        notify::Notifier,
    },
    log::{LogLine, LogViewer},
    quit_popup::QuitPopup,
    splash::Splash,
};

use color_eyre::Result;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
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
    area: Rect,
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
            area: Rect::default(),
        }
    }
}

impl HasFocus for HomeComponent {
    fn build(&self, builder: &mut FocusBuilder) {
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
    fn handle_tui_event(&mut self, tui_event: TuiEvent) -> Result<Option<Action>> {
        if let TuiEvent::Tick = tui_event {
            self.active_view = if self.flow_store.flows.is_empty() {
                ActiveView::Splash
            } else {
                ActiveView::FlowList
            }
        }
        Ok(None)
    }

    fn handle_action(&mut self, action: Action) -> ActionResult {
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

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        match self.active_view {
            ActiveView::Splash => self.splash.render(frame, area)?,
            ActiveView::FlowList => self.flow_list.render(frame, area)?,
        };

        self.fps_counter.render(frame, area)?;
        match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.render(frame, area)?,
            Some(ActivePopup::QuitPopup) => self.quit_popup.render(frame, area)?,
            Some(ActivePopup::FlowDetails) => self.flow_details.render(frame, area)?,
            Some(ActivePopup::LogViewer) => self.log_viewer.render(frame, area)?,
            None => {}
        };

        self.notifier.render(frame, area);
        Ok(())
    }

    fn children(&mut self) -> Vec<&mut dyn Component> {
        let mut children: Vec<&mut dyn Component> = vec![];
        match self.active_popup {
            Some(ActivePopup::ConfigEditor) => children.push(&mut self.config_editor),
            Some(ActivePopup::QuitPopup) => children.push(&mut self.quit_popup),
            Some(ActivePopup::FlowDetails) => children.push(&mut self.flow_details),
            Some(ActivePopup::LogViewer) => children.push(&mut self.log_viewer),
            None => {}
        }

        match self.active_view {
            ActiveView::Splash => children.push(&mut self.splash),
            ActiveView::FlowList => children.push(&mut self.flow_list),
        }
        children.push(&mut self.fps_counter);
        children
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
