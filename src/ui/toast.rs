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

#[derive(Debug, Clone)]
enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    message: String,
    kind: ToastKind,
    duration: Duration,
}

impl Toast {
    pub fn info<S: Into<String>>(msg: S) -> Self {
        Self {
            kind: ToastKind::Info,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn success<S: Into<String>>(msg: S) -> Self {
        Self {
            kind: ToastKind::Success,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn warning<S: Into<String>>(msg: S) -> Self {
        Self {
            kind: ToastKind::Warning,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn error<S: Into<String>>(msg: S) -> Self {
        Self {
            kind: ToastKind::Error,
            message: msg.into(),
            duration: Duration::from_secs(3),
        }
    }
}

struct ActiveToast {
    toast: Toast,
    created_at: Instant,
}

pub struct Toaster {
    receiver: Receiver<Toast>,
    toasts: VecDeque<ActiveToast>,
    max_visible: usize,
}

impl Toaster {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<Toast>(100);
        TOAST_SENDER.set(tx).unwrap();

        Self {
            receiver: rx,
            toasts: VecDeque::new(),
            max_visible: 5,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.created_at) < t.toast.duration);

        if self.toasts.len() >= self.max_visible {
            return;
        }

        while let Ok(toast) = self.receiver.try_recv() {
            self.toasts.push_back(ActiveToast {
                toast,
                created_at: Instant::now(),
            });
        }
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.update(); // Clear expired toasts

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

        for (idx, toast) in active_toasts.iter().enumerate() {
            let horizontal = Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(40), // Toast width
            ])
            .split(layout[idx]);

            let style = toast_style(&toast.toast.kind);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(style)
                .title(toast_title(&toast.toast.kind));

            let paragraph = Paragraph::new(toast.toast.message.clone())
                .block(block)
                .style(style)
                .alignment(Alignment::Center);

            frame.render_widget(Clear, horizontal[1]);
            frame.render_widget(paragraph, horizontal[1]);
        }
    }
}

fn toast_title(kind: &ToastKind) -> String {
    match kind {
        ToastKind::Info => "Info".to_string(),
        ToastKind::Success => "Success".to_string(),
        ToastKind::Warning => "Warning".to_string(),
        ToastKind::Error => "Error".to_string(),
    }
}

fn toast_style(kind: &ToastKind) -> Style {
    let colors = with_theme(|t| t.colors.clone());
    match kind {
        ToastKind::Info => Style::default().fg(colors.on_info).bg(colors.info),
        ToastKind::Success => Style::default().fg(colors.success).bg(colors.on_success),
        ToastKind::Warning => Style::default().fg(colors.warn).bg(colors.on_warn),
        ToastKind::Error => Style::default()
            .fg(colors.on_error)
            .bg(colors.error)
            .add_modifier(Modifier::BOLD),
    }
}

static TOAST_SENDER: OnceCell<Sender<Toast>> = OnceCell::new();

pub fn post_toast(toast: Toast) {
    trace!("Posting toast: {:?}", toast.message);
    if let Some(sender) = TOAST_SENDER.get() {
        let _ = sender.try_send(toast);
    } else {
        error!("Toast sender not initialized");
    }
}

#[macro_export]
macro_rules! toast_success {
    ($msg:expr) => {
        $crate::ui::toast::post_toast($crate::ui::toast::Toast::success($msg));
    };
}
#[macro_export]
macro_rules! toast_error {
    ($msg:expr) => {
        $crate::ui::toast::post_toast($crate::ui::toast::Toast::error($msg));
    };
}
#[macro_export]
macro_rules! toast_info {
    ($msg:expr) => {
        $crate::ui::toast::post_toast($crate::ui::toast::Toast::info($msg));
    };
}
#[macro_export]
macro_rules! toast_warn {
    ($msg:expr) => {
        $crate::ui::toast::post_toast($crate::ui::toast::Toast::warning($msg));
    };
}
