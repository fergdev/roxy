use ratatui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
};

use crate::config::Theme;

use std::cell::RefCell;

thread_local! {
    static CURRENT_THEME: RefCell<Theme> = RefCell::new(Theme::default());
}

pub fn set_theme(theme: Theme) {
    CURRENT_THEME.with(|t| *t.borrow_mut() = theme);
}

pub fn with_theme<F, R>(f: F) -> R
where
    F: FnOnce(&Theme) -> R,
{
    CURRENT_THEME.with(|t| f(&t.borrow()))
}

pub fn themed_block(title: &str) -> Block<'_> {
    let colors = with_theme(|t| t.colors.clone());
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.outline).bg(colors.surface))
        .style(Style::default().fg(colors.secondary).bg(colors.surface))
        .title(title)
        .title_style(
            Style::default()
                .fg(colors.secondary)
                .bg(colors.surface)
                .add_modifier(Modifier::BOLD),
        )
        .title_alignment(Alignment::Center)
}

pub fn themed_tabs(titles: Vec<Line<'_>>, selected: usize) -> Tabs<'_> {
    let colors = with_theme(|t| t.colors.clone());
    Tabs::new(titles)
        .block(themed_block("Config"))
        .highlight_style(Style::default().fg(colors.primary))
        .select(selected)
}

pub fn themed_table<'a, R, C>(rows: R, widths: C) -> Table<'a>
where
    R: IntoIterator,
    R::Item: Into<Row<'a>>,
    C: IntoIterator,
    C::Item: Into<Constraint>,
{
    let colors = with_theme(|t| t.colors.clone());

    let hl_style = Style::default()
        .fg(colors.on_primary)
        .bg(colors.primary)
        .add_modifier(Modifier::BOLD);

    Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.outline)),
        )
        .column_spacing(2)
        .row_highlight_style(hl_style)
}

pub fn themed_button(label: &str, selected: bool) -> Paragraph<'_> {
    let colors = with_theme(|t| t.colors.clone());
    let style = if selected {
        Style::default()
            .fg(colors.on_primary)
            .bg(colors.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.on_surface).bg(colors.surface)
    };

    Paragraph::new(label)
        .style(style)
        .alignment(Alignment::Center)
}

pub fn themed_info_block(message: &str) -> Paragraph<'_> {
    let colors = with_theme(|t| t.colors.clone());

    let style = Style::default()
        .fg(colors.secondary)
        .bg(colors.surface)
        .add_modifier(Modifier::BOLD | Modifier::ITALIC);

    Paragraph::new(
        Line::from(Span::styled(message.to_string(), style)).alignment(Alignment::Center),
    )
    .alignment(ratatui::layout::Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.outline))
            .style(Style::default().bg(colors.surface)),
    )
}

#[macro_export]
macro_rules! themed_line {
    ($text:expr) => {{
        $crate::ui::theme::with_theme(|t| {
            ratatui::text::Line::styled(
                $text.to_string(),
                ratatui::style::Style::default().fg(t.colors.on_surface),
            )
        })
    }};
}

#[macro_export]
macro_rules! themed_row {
    ($text:expr) => {{
        $crate::ui::theme::with_theme(|t| {
            ratatui::widgets::Row::new($text)
                .style(ratatui::style::Style::default().fg(t.colors.on_surface))
        })
    }};
}
