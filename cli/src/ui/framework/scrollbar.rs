use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::Color,
    symbols::scrollbar::Set,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

/// Render a horizontal scrollbar at the bottom of the area.
pub fn render_horizontal_scrollbar(frame: &mut Frame, area: Rect, horizontal: &mut ScrollbarState) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
        .symbols(Set {
            track: "-",
            thumb: "▮",
            begin: "<",
            end: ">",
        })
        .track_style(Color::Yellow)
        .begin_style(Color::Green)
        .end_style(Color::Red);

    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        }),
        horizontal,
    );
}
/// Render a vertical scrollbar on the right side of the area.
pub fn render_vertical_scrollbar(frame: &mut Frame, area: Rect, vertical: &mut ScrollbarState) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        vertical,
    );
}
