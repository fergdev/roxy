use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use color_eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};
use tokio::task::JoinHandle;
use tracing::error;

use crate::{
    event::Action,
    flow::{CertInfo, FlowStore, InterceptedRequest, InterceptedResponse},
};

use super::{component::Component, util::centered_rect};

struct State {
    request_lines: Vec<String>,
    response_lines: Vec<String>,
    cert_lines: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            request_lines: vec![],
            response_lines: vec![],
            cert_lines: vec![],
        }
    }
}

pub struct FlowDetails {
    flow_store: FlowStore,
    selected_flow: Option<i64>,
    state: Arc<Mutex<State>>,
    scroll: usize,
    tab_index: usize,
    listener_handle: JoinHandle<()>,
    flow_id_tx: tokio::sync::watch::Sender<Option<i64>>,
}

impl FlowDetails {
    pub fn new(flow_store: FlowStore) -> Self {
        let (tx, mut rx) = tokio::sync::watch::channel(None::<i64>);
        let state = Arc::new(Mutex::new(State::default()));

        let task_flow_store = flow_store.clone();
        let task_state = state.clone();
        let handle = tokio::spawn(async move {
            loop {
                let id_opt = (*rx.borrow_and_update()).clone();
                if let Some(flow_id) = id_opt {
                    let maybe_entry = task_flow_store.get_flow_by_id(flow_id).await;

                    let (req, resp, certs) = if let Some(entry) = maybe_entry {
                        let flow = entry.read().await;
                        (
                            render_request(&flow.request),
                            render_response(&flow.response),
                            render_certs(&flow.cert_info),
                        )
                    } else {
                        (
                            vec!["No request".into()],
                            vec!["No response".into()],
                            vec!["No certs".into()],
                        )
                    };

                    if let Ok(mut guard) = task_state.lock() {
                        guard.request_lines = req;
                        guard.response_lines = resp;
                        guard.cert_lines = certs;
                    }
                } else {
                    // TODO: suspicious
                    // Wait until rx changes before looping again
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        });

        Self {
            flow_store,
            selected_flow: None,
            state,
            scroll: 0,
            tab_index: 0,
            listener_handle: handle,
            flow_id_tx: tx,
        }
    }

    pub fn set_flow(&mut self, flow_id: i64) {
        self.selected_flow = Some(flow_id);
        self.flow_id_tx.send(Some(flow_id)).unwrap_or_else(|_| {
            error!("Failed to send flow ID, channel closed");
        });
    }

    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 3;
    }

    fn prev_tab(&mut self) {
        if self.tab_index == 0 {
            self.tab_index = 2;
        } else {
            self.tab_index = self.tab_index.wrapping_sub(1);
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll += 1;
    }
}

fn render_request(req: &Option<InterceptedRequest>) -> Vec<String> {
    let mut lines = vec!["== Request ==".to_string()];

    if let Some(req) = req {
        lines.push(req.line_pretty());
        for (k, v) in &req.headers {
            lines.push(format!("{}: {}", k, v));
        }
        if let Some(body) = &req.body {
            lines.push("".to_string());
            lines.push(body.clone());
        }
    } else {
        lines.push("(no request)".to_string());
    }

    lines
}

fn render_response(resp: &Option<InterceptedResponse>) -> Vec<String> {
    let mut lines = vec!["== Response ==".to_string()];

    if let Some(resp) = resp {
        lines.push(resp.request_line());
        for (k, v) in &resp.headers {
            lines.push(format!("{}: {}", k, v));
        }
        if let Some(body) = &resp.body {
            lines.push("".to_string());
            lines.push(body.clone());
        }
    } else {
        lines.push("(no response)".to_string());
    }

    lines
}

fn render_certs(certs: &Option<Vec<CertInfo>>) -> Vec<String> {
    let mut lines = vec!["== Certificates ==".to_string()];

    if let Some(certs) = certs {
        for cert in certs {
            lines.push(format!("Issuer: {}", cert.issuer));
            lines.push(format!("Subject: {}", cert.subject));
            lines.push(format!("Not Before: {}", cert.not_before));
            lines.push(format!("Not After: {}", cert.not_after));
            lines.push("".to_string());
        }
    } else {
        lines.push("(no certificates)".to_string());
    }

    lines
}

impl Component for FlowDetails {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Down => {
                self.scroll_down();
                Ok(None)
            }
            Action::Up => {
                self.scroll_up();
                Ok(None)
            }
            Action::Left => {
                self.prev_tab();
                Ok(None)
            }
            Action::Right => {
                self.next_tab();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        let popup_area = centered_rect(60, 60, area);

        let tab_titles = ["Request", "Response", "Certs"];
        let tabs = Tabs::new(tab_titles.map(Line::from).to_vec())
            .select(self.tab_index)
            .highlight_style(Style::default().bold());

        let state = self.state.lock().unwrap();

        let lines = match self.tab_index {
            0 => &state.request_lines,
            1 => &state.response_lines,
            2 => &state.cert_lines,
            _ => panic!("Invalid tab index"),
        };

        let text = lines
            .iter()
            .skip(self.scroll)
            .map(|s| Line::from(s.clone()))
            .collect::<Vec<_>>();

        let paragraph = Paragraph::new(Text::from(text))
            .scroll((self.scroll as u16, 0))
            .block(Block::default().title("Flow Details").borders(Borders::ALL));

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(popup_area);

        f.render_widget(Clear, popup_area);
        f.render_widget(tabs, layout[0]);
        f.render_widget(paragraph, layout[1]);
        Ok(())
    }
}

impl Drop for FlowDetails {
    fn drop(&mut self) {
        self.listener_handle.abort(); // cancel background task
    }
}
