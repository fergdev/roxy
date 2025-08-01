use ratatui::{Frame, layout::Rect, widgets::WidgetRef};
use tracing::debug;

pub struct CachedRender {
    last_hash: u64,
    last_area: Option<ratatui::layout::Rect>,
    widget: Option<Box<dyn ratatui::widgets::WidgetRef>>,
}

impl CachedRender {
    pub fn new() -> Self {
        Self {
            last_hash: 0,
            last_area: None,
            widget: None,
        }
    }

    pub fn render_if_changed<W: WidgetRef + 'static>(
        &mut self,
        f: &mut Frame,
        area: Rect,
        data: &str,
        make_widget: impl Fn() -> W,
    ) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        area.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.last_hash != current_hash || self.last_area != Some(area) {
            debug!("changed");
            self.widget = Some(Box::new(make_widget()));
            self.last_hash = current_hash;
            self.last_area = Some(area);
        }

        let widget = self.widget.as_ref().unwrap();
        widget.render_ref(area, f.buffer_mut());
    }
}

impl Default for CachedRender {
    fn default() -> Self {
        Self::new()
    }
}
