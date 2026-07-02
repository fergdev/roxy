use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use snowflake::SnowflakeIdGenerator;
use tracing::{debug, error};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(crate) struct ImageCache {
    inner: Arc<Mutex<ImageCacheInner>>,
}

struct ImageCacheInner {
    id_gen: SnowflakeIdGenerator,
    cache: HashMap<i64, Arc<Mutex<StatefulProtocol>>>,
}

impl ImageCache {
    pub(crate) fn new() -> Self {
        ImageCache {
            inner: Arc::new(Mutex::new(ImageCacheInner {
                id_gen: SnowflakeIdGenerator::new(1, 1),
                cache: HashMap::new(),
            })),
        }
    }

    pub(crate) fn render_image(&mut self, raw: &[u8]) -> Option<i64> {
        if let Ok(image) = image::load_from_memory(raw) {
            // TODO: make this configurable
            let mut picker = Picker::halfblocks();
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

    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, id: &i64) {
        if let Ok(guard) = self.inner.lock()
            && let Some(proto_arc) = guard.cache.get(id)
        {
            match proto_arc.lock() {
                Ok(mut proto) => {
                    let image = StatefulImage::default().resize(Resize::default());
                    frame.render_stateful_widget(image, area, &mut *proto);
                    return;
                }
                Err(e) => {
                    error!("Failed to lock image protocol for rendering {e}");
                }
            }
        }
        let para = Paragraph::new(Line::raw("Failed to render image"))
            .block(Block::default().title("Body").borders(Borders::ALL))
            .scroll((0, 0));
        frame.render_widget(para, area);
    }
}
