use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use once_cell::sync::OnceCell;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, HorizontalAlignment, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
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
}

impl Toaster {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<Toast>(100); // holds up to 100 toast events
        TOAST_SENDER.set(tx).unwrap();

        // let deque = Arc::new(Mutex::new(VecDeque::new()));

        Self {
            receiver: rx,
            toasts: VecDeque::new(),
        }
    }

    fn update(&mut self) {
        // drain new toasts from channel
        while let Ok(toast) = self.receiver.try_recv() {
            self.toasts.push_back(ActiveToast {
                toast,
                created_at: Instant::now(),
            });
        }

        // remove expired toasts
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.created_at) < t.toast.duration);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.update();

        if let Some(active) = self.toasts.front() {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(40), // Toast width
                ])
                .split(area);

            let toast_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(layout[1])[0];

            frame.render_widget(Clear, toast_area);

            let style = toast_style(&active.toast.kind);
            let block = Block::default()
                .title(toast_title(&active.toast.kind))
                .title_alignment(HorizontalAlignment::Center)
                .borders(Borders::ALL)
                .style(style);

            let text = Paragraph::new(active.toast.message.clone())
                .block(block)
                .alignment(Alignment::Center)
                .style(style);

            frame.render_widget(text, toast_area);
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
