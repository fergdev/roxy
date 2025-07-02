use std::{
    fs,
    sync::{Arc, Mutex},
};

use base64::{
    Engine, alphabet,
    engine::{GeneralPurpose, general_purpose},
};
use color_eyre::Result;
use kuchiki::{parse_html, traits::TendrilSink};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use tokio::task::JoinHandle;
use tracing::{debug, error};

use crate::{
    event::Action,
    flow::{CertInfo, Flow, FlowStore, InterceptedRequest, InterceptedResponse},
    themed_line,
};

use super::{
    component::Component,
    theme::{themed_block, themed_tabs},
    util::centered_rect,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Request,
    Response,
    Certs,
}

impl Tab {
    fn all() -> &'static [Tab] {
        &[Self::Request, Self::Response, Self::Certs]
    }

    fn title(&self) -> &'static str {
        match self {
            Tab::Request => "Request",
            Tab::Response => "Response",
            Tab::Certs => "Certs",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|&t| t == *self).unwrap_or(0)
    }

    fn prev(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == 0 {
            *all_tabs.last().unwrap()
        } else {
            all_tabs[index - 1]
        }
    }

    fn next(&self) -> Self {
        let all_tabs = Self::all();
        let index = self.index();
        if index == all_tabs.len() - 1 {
            *all_tabs.first().unwrap()
        } else {
            all_tabs[index + 1]
        }
    }
}

#[derive(Default)]
struct State {
    request_lines: Vec<String>,
    response_lines: Vec<String>,
    cert_lines: Vec<String>,
    flow: Option<Flow>, // HACK: Hack fest
}

pub struct FlowDetails {
    selected_flow: Option<i64>,
    state: Arc<Mutex<State>>,
    scroll: usize,
    tab: Tab,
    listener_handle: JoinHandle<()>,
    flow_id_tx: tokio::sync::watch::Sender<Option<i64>>,
    proto: Option<StatefulProtocol>,
}

impl FlowDetails {
    pub fn new(flow_store: FlowStore) -> Self {
        let (tx, mut rx) = tokio::sync::watch::channel(None::<i64>);
        let state = Arc::new(Mutex::new(State::default()));

        let task_flow_store = flow_store.clone();
        let task_state = state.clone();
        let handle = tokio::spawn(async move {
            loop {
                let id_opt = *rx.borrow_and_update();
                if let Some(flow_id) = id_opt {
                    let maybe_entry = task_flow_store.get_flow_by_id(flow_id).await;

                    let (req, resp, certs, flow) = if let Some(entry) = maybe_entry {
                        let flow = entry.read().await;
                        (
                            render_request(&flow.request),
                            render_response(&flow.response),
                            render_certs(&flow.cert_info),
                            Some(flow.clone()),
                        )
                    } else {
                        (
                            vec!["No request".into()],
                            vec!["No response".into()],
                            vec!["No certs".into()],
                            None,
                        )
                    };

                    if let Ok(mut guard) = task_state.lock() {
                        guard.request_lines = req;
                        guard.response_lines = resp;
                        guard.cert_lines = certs;
                        guard.flow = flow
                    }
                }
            }
        });

        Self {
            selected_flow: None,
            state,
            scroll: 0,
            tab: Tab::Request,
            listener_handle: handle,
            flow_id_tx: tx,
            proto: None,
        }
    }

    pub fn set_flow(&mut self, flow_id: i64) {
        self.selected_flow = Some(flow_id);
        self.flow_id_tx.send(Some(flow_id)).unwrap_or_else(|_| {
            error!("Failed to send flow ID, channel closed");
        });
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }

    fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
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
    let mut lines = vec![];

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
    let mut lines = vec![];

    if let Some(resp) = resp {
        lines.push(resp.request_line());
        for (k, v) in &resp.headers {
            lines.push(format!("{}: {}", k, v));
        }
        let content_type = resp
            .headers
            .get("Content-Type")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(body) = &resp.body {
            if content_type.contains("xml") {
                lines.extend(pretty_print_xml(String::from_utf8_lossy(body).as_ref()));
            } else if content_type.contains("html") {
                lines.extend(pretty_print_html(String::from_utf8_lossy(body).as_ref()));
            } else if content_type.contains("json") {
                lines.extend(pretty_print_json(String::from_utf8_lossy(body).as_ref()));
            } else {
                lines.push(String::from_utf8_lossy(body).to_string());
            }
        }
    } else {
        lines.push("(no response)".to_string());
    }

    lines
}

fn render_certs(certs: &Option<Vec<CertInfo>>) -> Vec<String> {
    let mut lines = vec![];

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

        f.render_widget(Clear, popup_area);

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(popup_area);
        let tab_titles: Vec<Line> = Tab::all().iter().map(|t| Line::raw(t.title())).collect();
        let tab_index = self.tab.index();

        let tabs = themed_tabs(tab_titles, tab_index);
        f.render_widget(tabs, layout[0]);

        let state = self.state.lock().unwrap();

        debug!(
            "Rendering FlowDetails for flow ID: {:?}",
            self.selected_flow
        );
        if self.tab == Tab::Request {
            let lines = match self.tab {
                Tab::Request => &state.request_lines,
                Tab::Response => &state.response_lines,
                Tab::Certs => &state.cert_lines,
            };
            debug!("Rendering FlowDetails for tab: {:?}", self.tab);

            let text = lines
                .iter()
                .skip(self.scroll)
                .map(|s| themed_line!(s))
                .collect::<Vec<_>>();

            let paragraph = Paragraph::new(Text::from(text))
                .scroll((self.scroll as u16, 0))
                .block(themed_block("Flow Details"));

            let layout =
                Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(popup_area);

            f.render_widget(paragraph, layout[1]);
        } else if let Some(flow) = &state.flow {
            debug!("Rendering FlowDetails for flow ID: {}", flow.id);
            if let Some(resp) = &flow.response {
                let content_type = resp
                    .headers
                    .get("Content-Type")
                    .map(String::as_str)
                    .unwrap_or("text/plain");

                match content_type {
                    ct if ct.contains("json") => {
                        if let Some(body) = &resp.body {
                            let lines = pretty_print_json("blah").join("\n");
                            let para = Paragraph::new(lines)
                                .block(Block::default().title("JSON").borders(Borders::ALL));
                            f.render_widget(para, popup_area);
                        }
                    }
                    // ct if ct.contains("html") => {
                    //     // Render HTML as styled text (or raw)
                    // }
                    ct if ct.contains("png") => {
                        debug!("Rendering PNG image in FlowDetails");
                        if let Some(proto) = &mut self.proto {
                            let image = StatefulImage::default();

                            error!("Image size: ");
                            f.render_stateful_widget(image, popup_area, proto);
                        } else {
                            if let Some(body) = &resp.body {
                                let preview = String::from_utf8_lossy(body)
                                    .to_string()
                                    .chars()
                                    .take(50)
                                    .collect::<String>();

                                let path = "/tmp/dump.png"; // or .jpg, .pdf, etc.
                                if let Err(e) = fs::write(path, &body) {
                                } else {
                                    println!("Wrote decoded body to: {path}");
                                }
                                debug!("Body preview: {:?}", preview);
                                // debug!(
                                //     "Body preview: {:?}",
                                //     &body.chars().take(50).collect::<String>()
                                // );
                                // let decoded = GeneralPurpose::new(
                                //     &alphabet::URL_SAFE,
                                //     general_purpose::NO_PAD,
                                // )
                                // .decode(body);
                                // match decoded {
                                //     Ok(bytes) => {
                                // if let Ok(image) = image::load_from_memory(&bytes) {
                                if let Ok(image) = image::load_from_memory(&body) {
                                    debug!("Loaded image with size: ");
                                    let mut picker = Picker::from_fontsize((8, 12));
                                    let mut proto = picker.new_resize_protocol(image);
                                    self.proto = Some(proto);
                                } else {
                                    error!("Failed to load image from memory");
                                }
                                //     }
                                //     Err(_) => {
                                //         error!("Failed to decode base64 PNG body: ");
                                //     }
                                // }
                            } else {
                                error!("No body found for PNG response");
                            }
                        }
                    }
                    _ => {
                        // fallback raw body
                        if let Some(body) = &resp.body {
                            let para = Paragraph::new("blah".to_string())
                                .block(Block::default().title("Body").borders(Borders::ALL));
                            f.render_widget(para, popup_area);
                        }
                    }
                }
            } else {
                let empty =
                    Paragraph::new("(no response)").block(Block::default().borders(Borders::ALL));
                f.render_widget(empty, popup_area);
            }
        }
        Ok(())
    }
}

impl Drop for FlowDetails {
    fn drop(&mut self) {
        self.listener_handle.abort();
    }
}

fn pretty_print_xml(raw: &str) -> Vec<String> {
    match xmltree::Element::parse(raw.as_bytes()) {
        Ok(elem) => {
            let mut out = Vec::new();
            let mut buffer = Vec::new();
            if elem
                .write_with_config(
                    &mut buffer,
                    xmltree::EmitterConfig::new().perform_indent(true),
                )
                .is_ok()
            {
                if let Ok(s) = String::from_utf8(buffer) {
                    out.extend(s.lines().map(|line| line.to_string()));
                }
            }
            out
        }
        Err(_) => vec!["<invalid xml>".into()],
    }
}

pub fn pretty_print_html(raw: &str) -> Vec<String> {
    let parser = parse_html().from_utf8();
    let document = parser.read_from(&mut raw.as_bytes());

    match document {
        Ok(doc) => {
            let mut out = Vec::new();
            let mut buffer = Vec::new();
            if doc.serialize(&mut buffer).is_ok() {
                if let Ok(s) = String::from_utf8(buffer) {
                    out.extend(s.lines().map(|line| line.to_string()));
                }
            }
            out
        }
        Err(_) => vec!["<invalid html>".into()],
    }
}

fn pretty_print_json(raw: &str) -> Vec<String> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) => {
            let pretty = serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.to_string());
            pretty.lines().map(|line| line.to_string()).collect()
        }
        Err(e) => {
            error!("Failed to parse JSON: {}", e);
            error!("{}", raw);

            vec!["<invalid json>".into()]
        }
    }
}
