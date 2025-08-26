use bytes::Bytes;
use color_eyre::Result;
use rat_focus::{FocusFlag, HasFocus};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use roxy_shared::content::ContentType;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::{mpsc, watch};
use tracing::debug;
use x509_parser::nom::HexDisplay;

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
    ui::framework::{
        component::{ActionResult, Component},
        theme::themed_block,
    },
};

fn render_plain_text(body: &Bytes) -> Vec<Line<'static>> {
    let utf = String::from_utf8_lossy(body);
    utf.lines()
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
                        ContentType::Json => Body::Text(highlight_json(body)),
                        ContentType::Svg | ContentType::Xml => Body::Text(pretty_print_xml(&body)), // TODO:
                        // can we render svg
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
                            let line = vec![hex.into()];
                            Body::Text(line)
                        }
                        ContentType::Text => {
                            let lines = render_plain_text(&body);
                            Body::Text(lines)
                        }
                    },
                    None => {
                        if body.is_empty() {
                            Body::None
                        } else {
                            let lines = render_plain_text(&body);
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
            image_cache: ic,
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
                    let len = self.state.borrow().len() + 5;

                    self.scroll += 1;
                    if self.scroll > len {
                        self.scroll = len;
                    }
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
                    .wrap(Wrap { trim: false })
                    .block(themed_block(Some("Body"), self.focus.get()))
                    .scroll((self.scroll, 0));
                f.render_widget(para, area);
            }
            Body::Image(ref id) => {
                if let Some(id) = id {
                    return self.image_cache.render(f, area, id);
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

#[derive(Clone)]
struct ImageCache {
    inner: Arc<Mutex<ImageCacheInner>>,
}

struct ImageCacheInner {
    id_gen: SnowflakeIdGenerator,
    cache: HashMap<i64, Arc<Mutex<StatefulProtocol>>>,
}

impl ImageCache {
    fn new() -> Self {
        ImageCache {
            inner: Arc::new(Mutex::new(ImageCacheInner {
                id_gen: SnowflakeIdGenerator::new(1, 1),
                cache: HashMap::new(),
            })),
        }
    }

    fn render_image(&mut self, raw: &[u8]) -> Option<i64> {
        if let Ok(image) = image::load_from_memory(raw) {
            debug!("Loaded image with size: ");
            // TODO: make this configurable
            let mut picker = Picker::from_fontsize((9, 20));
            picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
            let proto = picker.new_resize_protocol(image);

            if let Ok(mut guard) = self.inner.lock() {
                let id = guard.id_gen.generate();
                guard.cache.insert(id, Arc::new(Mutex::new(proto)));
                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, id: &i64) -> Result<()> {
        if let Ok(guard) = self.inner.lock()
            && let Some(proto_arc) = guard.cache.get(id)
        {
            match proto_arc.lock() {
                Ok(mut proto) => {
                    let image = StatefulImage::default().resize(Resize::default());
                    f.render_stateful_widget(image, area, &mut *proto);
                    return Ok(());
                }
                Err(_) => {
                    eprintln!("Failed to lock image protocol for rendering");
                }
            }
        }
        let para = Paragraph::new(Line::raw("Failed to render image"))
            .block(Block::default().title("Body").borders(Borders::ALL))
            .scroll((0, 0));
        f.render_widget(para, area);
        Ok(())
    }
}
