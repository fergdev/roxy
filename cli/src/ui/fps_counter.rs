use std::time::Instant;

use color_eyre::Result;
use rat_focus::FocusFlag;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::{action::Action, tui::TuiEvent};

use super::framework::{
    component::{ActionResult, Component},
    theme::themed_info_block,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FpsCounter {
    area: Rect,
    focus: FocusFlag,

    last_tick_update: Instant,
    tick_count: u32,
    ticks_per_second: f64,

    last_frame_update: Instant,
    frame_count: u32,
    frames_per_second: f64,

    visible: bool,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("FpsCounter"),
            last_tick_update: Instant::now(),
            tick_count: 0,
            ticks_per_second: 0.0,
            last_frame_update: Instant::now(),
            frame_count: 0,
            frames_per_second: 0.0,
            visible: false,
        }
    }

    fn app_tick(&mut self) {
        self.tick_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_tick_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.ticks_per_second = self.tick_count as f64 / elapsed;
            self.last_tick_update = now;
            self.tick_count = 0;
        }
    }

    fn render_tick(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_frame_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.frames_per_second = self.frame_count as f64 / elapsed;
            self.last_frame_update = now;
            self.frame_count = 0;
        }
    }
}

impl Component for FpsCounter {
    fn dispatch_tui_events(&mut self, tui_event: TuiEvent) -> Result<Option<Action>> {
        match tui_event {
            TuiEvent::Tick => {
                self.app_tick();
                Ok(None)
            }
            TuiEvent::Render => {
                self.render_tick();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    fn handle_action(&mut self, action: Action) -> ActionResult {
        if let Action::FpsView = action {
            self.visible = !self.visible
        };
        ActionResult::Ignored
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = area;
        if !self.visible {
            return Ok(());
        }
        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

        let horizontal =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(vertical[0]);

        let message = format!(
            "{:.2} ticks/sec, {:.2} FPS",
            self.ticks_per_second, self.frames_per_second
        );
        let widget = themed_info_block(&message);

        frame.render_widget(widget, horizontal[1]);
        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
