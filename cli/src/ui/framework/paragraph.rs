use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::ui::framework::theme::{tertiary_text, themed_block};

pub fn kv_paragraph(
    state: &[(String, String)],
    frame: &mut Frame,
    area: Rect,
    title: Option<&str>,
    focused: bool,
    scroll: (u16, u16),
) {
    let lines = state
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k}:"), tertiary_text()),
                Span::raw(v),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(themed_block(title, focused))
            .scroll(scroll),
        area,
    );
}
