use bytes::Bytes;
use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use snowflake::SnowflakeIdGenerator;
use tokio::sync::{mpsc, watch};
use tracing::debug;

use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Mutex},
};

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
    event::Action,
    flow::ContentType,
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_block,
    },
};

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
}

pub struct FlowDetailsBody {
    state: watch::Receiver<UiState>,
    focus: FocusFlag,
    scroll: u16,
}

impl FlowDetailsBody {
    pub fn new(mut body_rx: mpsc::Receiver<(ContentType, Bytes)>) -> Self {
        let (ui_tx, ui_rx) = watch::channel(UiState::default());

        tokio::spawn(async move {
            while let Some((content_type, mut body)) = body_rx.recv().await {
                let lines = match content_type {
                    ContentType::Json => Body::Text(highlight_json(body)),
                    ContentType::Xml => Body::Text(pretty_print_xml(&body)),
                    ContentType::Html => {
                        let mut cursor = Cursor::new(&mut body);
                        let lines = highlight_html_dom(&mut cursor);
                        Body::Text(lines)
                    }
                    ContentType::Toml => Body::Text(highlight_toml(&body)),
                    ContentType::Yaml => Body::Text(pretty_print_yaml(&body)),
                    ContentType::Csv => Body::Text(render_csv(&body)),
                    ContentType::Tsv => Body::Text(render_tsv(&body)),
                    ContentType::Md => Body::Text(render_markdown(&body)),
                    ContentType::Png => Body::Image(render_image(&body)),
                    ContentType::Gif => Body::Image(render_image(&body)),
                    ContentType::Jpeg => Body::Image(render_image(&body)),
                    ContentType::Webp => Body::Image(render_image(&body)),
                    ContentType::XIcon => Body::Image(render_image(&body)),
                    ContentType::Bmp => Body::Image(render_image(&body)),
                    ContentType::Text => {
                        let utf = String::from_utf8_lossy(&body);
                        let lines = utf
                            .lines()
                            .map(|line| Line::from(line.to_string()))
                            .collect::<Vec<Line>>();
                        Body::Text(lines)
                    }
                    ContentType::Unknown => {
                        if body.is_empty() {
                            Body::None
                        } else {
                            debug!("Unknown content type, treating as text");
                            let utf = String::from_utf8_lossy(&body);
                            let lines = utf
                                .lines()
                                .map(|line| Line::from(line.to_string()))
                                .collect::<Vec<Line>>();
                            Body::Text(lines)
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
            focus: rat_focus::FocusFlag::named("FlowBody"),
            scroll: 0,
        }
    }
}

impl HasFocus for FlowDetailsBody {
    fn build(&self, builder: &mut rat_focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        Rect::default()
    }
}

impl Component for FlowDetailsBody {
    fn update(&mut self, action: Action) -> ActionResult {
        if self.focus.get() {
            match action {
                Action::Up => {
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }

                    ActionResult::Consumed
                }
                Action::Down => {
                    self.scroll += 1;
                    ActionResult::Consumed
                }
                _ => ActionResult::Ignored,
            }
        } else {
            ActionResult::Ignored
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        if self.state.has_changed().unwrap_or(true) {
            self.scroll = 0;
        }
        match self.state.borrow_and_update().data {
            Body::None => {
                let para = Paragraph::new("No body")
                    .block(themed_block(Some("Body"), self.focus.get()))
                    .scroll((0, 0));
                f.render_widget(para, area);
            }
            Body::Text(ref lines) => {
                let para = Paragraph::new(lines.to_owned())
                    .block(themed_block(Some("Body"), self.focus.get()))
                    .scroll((self.scroll, 0));
                f.render_widget(para, area);
            }
            Body::Image(ref id) => {
                if let Some(id) = id {
                    let cache = IMAGE_CACHE.lock().unwrap();
                    if let Some(proto_arc) = cache.get(id) {
                        match proto_arc.lock() {
                            Ok(mut proto) => {
                                let image = StatefulImage::default().resize(Resize::default());
                                f.render_stateful_widget(image, area, &mut *proto);
                            }
                            Err(_) => {
                                eprintln!("Failed to lock image protocol for rendering");
                            }
                        }
                    }
                } else {
                    let para = Paragraph::new(Line::raw("Failed to render image"))
                        .block(Block::default().title("Body").borders(Borders::ALL))
                        .scroll((0, 0));
                    f.render_widget(para, area);
                }
            }
        }

        Ok(())
    }
}

static ID_GEN: Lazy<Arc<Mutex<SnowflakeIdGenerator>>> =
    Lazy::new(|| Arc::new(Mutex::new(SnowflakeIdGenerator::new(1, 1))));

fn generate_id() -> i64 {
    let mut generator = ID_GEN.lock().unwrap();
    generator.generate()
}

fn render_image(raw: &[u8]) -> Option<i64> {
    if let Ok(image) = image::load_from_memory(&raw) {
        debug!("Loaded image with size: ");
        // TODO: make this configurable
        let mut picker = Picker::from_fontsize((9, 20));
        picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
        let proto = picker.new_resize_protocol(image);

        let id = generate_id();
        cache_image(id, proto);
        Some(id)
    } else {
        None
    }
}

// TODO: remove from cache
pub static IMAGE_CACHE: Lazy<Mutex<HashMap<i64, Arc<Mutex<StatefulProtocol>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn cache_image(id: i64, proto: StatefulProtocol) {
    IMAGE_CACHE
        .lock()
        .unwrap()
        .insert(id, Arc::new(Mutex::new(proto)));
}
