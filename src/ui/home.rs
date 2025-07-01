use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{app::TableColors, config::Config, event::Action, flow::FlowStore, tui::Event};

use super::{
    component::Component, config_editor::ConfigEditor, flow_details::FlowDetails,
    flow_list::FlowList, log::LogViewer, quit_popup::QuitPopup, splash::Splash,
};

use color_eyre::Result;
use ratatui::{Frame, layout::Rect, style::palette::tailwind::ZINC};

pub struct HomeComponent {
    flow_store: FlowStore,
    active_view: ActiveView,
    active_popup: Option<ActivePopup>,
    splash: Splash,
    flow_list: FlowList,
    flow_details: FlowDetails,
    config_editor: ConfigEditor,
    quit_popup: QuitPopup,
    log_viewer: LogViewer,
}

impl HomeComponent {
    pub fn new(
        _config: Config,
        flow_store: FlowStore,
        log_buffer: Arc<Mutex<VecDeque<String>>>,
    ) -> Self {
        let splash = Splash::new(6969);
        let flow_list = FlowList::new(flow_store.clone(), TableColors::new(&ZINC));
        Self {
            flow_store: flow_store.clone(),
            active_view: ActiveView::Splash,
            active_popup: None,
            splash,
            flow_list,
            config_editor: ConfigEditor::new(),
            quit_popup: QuitPopup::new(),
            flow_details: FlowDetails::new(flow_store.clone()),
            log_viewer: LogViewer::new(log_buffer),
        }
    }

    pub fn set_active_view(&mut self, view: ActiveView) {
        self.active_view = view;
    }

    pub fn get_active_view(&self) -> ActiveView {
        self.active_view
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
    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        let action = match event {
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            Event::Tick => {
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

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let res = match self.active_view {
            ActiveView::Splash => self.splash.update(action.clone())?,
            ActiveView::FlowList => self.flow_list.update(action.clone())?,
        };

        if let Some(res) = res {
            return Ok(Some(res));
        }

        let res = match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.update(action.clone())?,
            Some(ActivePopup::QuitPopup) => self.quit_popup.update(action.clone())?,
            Some(ActivePopup::FlowDetails) => self.flow_details.update(action.clone())?,
            Some(ActivePopup::LogViewer) => self.log_viewer.update(action.clone())?,
            None => None,
        };

        if let Some(res) = res {
            return Ok(Some(res));
        }

        match action {
            Action::LogView => {
                self.active_popup = Some(ActivePopup::LogViewer);
                Ok(None)
            }
            Action::EditConfig => {
                self.active_popup = Some(ActivePopup::ConfigEditor);
                Ok(None)
            }
            Action::Back => {
                match self.active_popup {
                    Some(_) => {
                        self.active_popup = None; // Close the popup
                        Ok(None)
                    }
                    _ => {
                        self.active_popup = Some(ActivePopup::QuitPopup);
                        Ok(None)
                    }
                }
            }
            Action::Select => {
                if let Some(id) = self.flow_list.selected_id() {
                    self.flow_details.set_flow(id);
                    self.active_popup = Some(ActivePopup::FlowDetails);
                };
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let res = match self.active_view {
            ActiveView::Splash => self.splash.render(f, area),
            ActiveView::FlowList => self.flow_list.render(f, area),
        };
        match self.active_popup {
            Some(ActivePopup::ConfigEditor) => self.config_editor.render(f, area),
            Some(ActivePopup::QuitPopup) => self.quit_popup.render(f, area),
            Some(ActivePopup::FlowDetails) => self.flow_details.render(f, area),
            Some(ActivePopup::LogViewer) => self.log_viewer.render(f, area),
            None => res,
        }
    }
}
