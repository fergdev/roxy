use bytes::Bytes;
use color_eyre::Result;
use crossterm::event::{MouseEvent, MouseEventKind};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use roxy_shared::content::ContentType;
use tokio::sync::{mpsc, watch};
use tracing::{debug, info};
use x509_parser::nom::HexDisplay;

use std::io::Cursor;

use super::{
    csv::{render_csv, render_tsv},
    html::highlight_html_dom,
    json::highlight_json,
    markdown::render_markdown,
    toml::highlight_toml,
    xml::pretty_print_xml,
    yaml::pretty_print_yaml,
};

use crate::{
    action::Action,
    ui::{
        flow::body::image_cache::ImageCache,
        framework::{
            component::{ActionResult, Component},
            theme::themed_block,
        },
    },
};

fn render_plain_text(body: &Bytes) -> Vec<Line<'static>> {
    String::from_utf8_lossy(body)
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<Line>>()
}

struct UiState {
    data: Body,
}

enum Body {
    None,
    Text(Vec<Line<'static>>), // HACK: yeah this needs to be done properly
    Image(Option<i64>),
}

impl UiState {
    fn default() -> Self {
        Self { data: Body::None }
    }

    fn len(&self) -> u16 {
        match &self.data {
            Body::None => 0,
            Body::Text(lines) => (lines.len() + 1) as u16,
            Body::Image(_) => 0,
        }
    }
}

pub struct FlowDetailsBody {
    state: watch::Receiver<UiState>,
    image_cache: ImageCache,
    focus: FocusFlag,
    area: Rect,
    scroll: u16,
}

impl FlowDetailsBody {
    pub fn new(mut body_rx: mpsc::Receiver<(Option<ContentType>, Bytes)>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        let ic = ImageCache::new();
        let mut image_cache = ic.clone();

        tokio::spawn(async move {
            while let Some((content_type, mut body)) = body_rx.recv().await {
                let lines = match content_type {
                    Some(ct) => match ct {
                        ContentType::Json => Body::Text(highlight_json(&body)),
                        ContentType::Svg | ContentType::Xml => Body::Text(pretty_print_xml(&body)),
                        ContentType::Html => {
                            let mut cursor = Cursor::new(&mut body);
                            match highlight_html_dom(&mut cursor) {
                                Ok(lines) => Body::Text(lines),
                                Err(_) => Body::None,
                            }
                        }
                        ContentType::Toml => Body::Text(highlight_toml(&body)),
                        ContentType::Yaml => Body::Text(pretty_print_yaml(&body)),
                        ContentType::Csv => {
                            Body::Text(render_csv(&body).unwrap_or(render_plain_text(&body)))
                        }
                        ContentType::Tsv => {
                            Body::Text(render_tsv(&body).unwrap_or(render_plain_text(&body)))
                        }
                        ContentType::Md => Body::Text(render_markdown(&body)),
                        ContentType::Png => Body::Image(image_cache.render_image(&body)),
                        ContentType::Gif => Body::Image(image_cache.render_image(&body)),
                        ContentType::Jpeg => Body::Image(image_cache.render_image(&body)),
                        ContentType::Webp => Body::Image(image_cache.render_image(&body)),
                        ContentType::XIcon => Body::Image(image_cache.render_image(&body)),
                        ContentType::Bmp => Body::Image(image_cache.render_image(&body)),
                        ContentType::OctetStream => {
                            let hex = body.to_hex(8);
                            Body::Text(vec![hex.into()])
                        }
                        ContentType::Text => Body::Text(render_plain_text(&body)),
                    },
                    None => {
                        if body.is_empty() {
                            Body::None
                        } else {
                            Body::Text(render_plain_text(&body))
                        }
                    }
                };

                ui_tx.send(UiState { data: lines }).unwrap_or_else(|e| {
                    debug!("Failed to send UI state update: {}", e);
                });
            }
        });
        Self {
            state: ui_rx,
            image_cache: ic,
            focus: FocusFlag::new().with_name("FlowBody"),
            area: Rect::default(),
            scroll: 0,
        }
    }
}

impl HasFocus for FlowDetailsBody {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl Component for FlowDetailsBody {
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        if !self.focus.get() {
            return Ok(Some(Action::FocusReq(self.focus.id())));
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            MouseEventKind::ScrollDown => {
                self.scroll = self.scroll.saturating_add(1);
            }
            _ => {
                // nothing
            }
        }
        Ok(None)
    }
    fn handle_action(&mut self, action: Action) -> ActionResult {
        if self.focus.get() {
            match action {
                Action::Up => {
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }

                    ActionResult::Consumed
                }
                Action::Down => {
                    let len = self.state.borrow().len() + 5;

                    if self.scroll > len {
                        self.scroll = len;
                    }
                    ActionResult::Consumed
                }
                Action::PageUp => {
                    if self.scroll > 0 {
                        self.scroll = self.scroll.saturating_sub(self.area.height);
                    }
                    ActionResult::Consumed
                }
                Action::PageDown => {
                    self.scroll = self.scroll.saturating_add(self.area.height);
                    ActionResult::Consumed
                }
                Action::Top => {
                    self.scroll = 0;
                    ActionResult::Consumed
                }
                Action::Bottom => {
                    self.scroll = u16::MAX;
                    ActionResult::Consumed
                }
                _ => ActionResult::Ignored,
            }
        } else {
            ActionResult::Ignored
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.area = area;
        if self.state.has_changed().unwrap_or(true) {
            self.scroll = 0;
        }
        match self.state.borrow_and_update().data {
            Body::None => {
                let para = Paragraph::new("No body")
                    .block(themed_block(Some("Body"), self.focus.get()))
                    .scroll((0, 0));
                frame.render_widget(para, area);
            }
            Body::Text(ref lines) => {
                let height = area.height;
                let len = lines.len() as u16;
                let max_scroll_index = len.saturating_sub(height);

                // TODO: work
                // +5 is an arbitrary number to compensate for wrapping, caclulate this from the
                // length of the lines
                self.scroll = self.scroll.clamp(0, max_scroll_index + 5);

                let para = Paragraph::new(lines.to_owned())
                    .wrap(Wrap { trim: false })
                    .block(themed_block(Some("Body"), self.focus.get()))
                    .scroll((self.scroll, 0));
                frame.render_widget(para, area);
            }
            Body::Image(ref id) => {
                if let Some(id) = id {
                    return self.image_cache.render(frame, area, id);
                } else {
                    let para = Paragraph::new(Line::raw("Failed to render image"))
                        .block(Block::default().title("Body").borders(Borders::ALL))
                        .scroll((0, 0));
                    frame.render_widget(para, area);
                }
            }
        }

        Ok(())
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }
}
