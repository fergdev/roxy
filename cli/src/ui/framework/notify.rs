use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use once_cell::sync::OnceCell;
use ratatui::widgets::Clear;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, trace};

use super::theme::with_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    message: String,
    level: Level,
    duration: Duration,
}

impl Notification {
    pub fn trace<S: Into<String>>(msg: S) -> Self {
        Self {
            level: Level::Trace,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }
    pub fn debug<S: Into<String>>(msg: S) -> Self {
        Self {
            level: Level::Debug,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }
    pub fn info<S: Into<String>>(msg: S) -> Self {
        Self {
            level: Level::Info,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn warning<S: Into<String>>(msg: S) -> Self {
        Self {
            level: Level::Warning,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn error<S: Into<String>>(msg: S) -> Self {
        Self {
            level: Level::Error,
            message: msg.into(),
            duration: Duration::from_secs(10), // TODO: make configurable
        }
    }
}

struct ActiveNotification {
    notification: Notification,
    created_at: Instant,
}

pub struct Notifier {
    receiver: Receiver<Notification>,
    toasts: VecDeque<ActiveNotification>,
    max_visible: usize,
    level: Level,
}

impl Notifier {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<Notification>(100);
        TOAST_SENDER.set(tx).unwrap();

        Self {
            receiver: rx,
            toasts: VecDeque::new(),
            max_visible: 5,
            level: Level::Info,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.created_at) < t.notification.duration);

        if self.toasts.len() >= self.max_visible {
            return;
        }

        while let Ok(notification) = self.receiver.try_recv() {
            if notification.level < self.level {
                continue;
            }
            self.toasts.push_back(ActiveNotification {
                notification,
                created_at: Instant::now(),
            });
        }
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.update();

        let active_toasts: Vec<_> = self.toasts.iter().take(self.max_visible).collect();
        if active_toasts.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                active_toasts
                    .iter()
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        for (idx, notification) in active_toasts.iter().enumerate() {
            let horizontal =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(40)]).split(layout[idx]);

            let style = toast_style(&notification.notification.level);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(style)
                .title(toast_title(&notification.notification.level))
                .title_alignment(Alignment::Center);

            let paragraph = Paragraph::new(notification.notification.message.clone())
                .block(block)
                .style(style)
                .alignment(Alignment::Center);

            frame.render_widget(Clear, horizontal[1]);
            frame.render_widget(paragraph, horizontal[1]);
        }
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

fn toast_title(level: &Level) -> String {
    match level {
        Level::Trace => "Trace".to_string(),
        Level::Debug => "Debug".to_string(),
        Level::Info => "Info".to_string(),
        Level::Warning => "Warning".to_string(),
        Level::Error => "Error".to_string(),
    }
}

fn toast_style(level: &Level) -> Style {
    let colors = with_theme(|t| t.colors.clone());
    match level {
        Level::Trace => Style::default().fg(colors.trace).bg(colors.surface),
        Level::Debug => Style::default().fg(colors.debug).bg(colors.surface),
        Level::Info => Style::default().fg(colors.info).bg(colors.surface),
        Level::Warning => Style::default().fg(colors.warn).bg(colors.surface),
        Level::Error => Style::default()
            .fg(colors.error)
            .bg(colors.surface)
            .add_modifier(Modifier::BOLD),
    }
}

static TOAST_SENDER: OnceCell<Sender<Notification>> = OnceCell::new();

pub fn post_notification(notification: Notification) {
    trace!("Posting notification: {:?}", notification.message);
    if let Some(sender) = TOAST_SENDER.get() {
        let _ = sender.try_send(notification);
    } else {
        error!("Notification sender not initialized");
    }
}

#[macro_export]
macro_rules! notify_trace {
    ($($arg:tt)*) => {
        $crate::ui::framework::notify::post_notification($crate::ui::framework::notify::Notification::trace(format!($($arg)*)))
    };
}
#[macro_export]
macro_rules! notify_debug {
    ($($arg:tt)*) => {
        $crate::ui::framework::notify::post_notification($crate::ui::framework::notify::Notification::debug(format!($($arg)*)))
    };
}
#[macro_export]
macro_rules! notify_error {
    ($($arg:tt)*) => {
        $crate::ui::framework::notify::post_notification($crate::ui::framework::notify::Notification::error(format!($($arg)*)))
    };
}
#[macro_export]
macro_rules! notify_info {
    ($($arg:tt)*) => {
        $crate::ui::framework::notify::post_notification($crate::ui::framework::notify::Notification::info(format!($($arg)*)))
    };
}
#[macro_export]
macro_rules! notify_warn {
    ($($arg:tt)*) => {
        $crate::ui::framework::notify::post_notification($crate::ui::framework::notify::Notification::warning(format!($($arg)*)))
    };
}
