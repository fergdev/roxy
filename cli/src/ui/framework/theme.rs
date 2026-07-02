use crate::config::Theme;
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, Tabs},
};
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

pub fn themed_block(title: Option<&str>, has_focus: bool) -> Block<'_> {
    with_theme(|theme| {
        let colors = &theme.colors;

        let mut title_style = Style::default().fg(colors.secondary).bg(colors.surface);
        title_style = if has_focus {
            title_style.add_modifier(Modifier::BOLD)
        } else {
            title_style
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(if has_focus {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(if has_focus {
                Style::default().fg(colors.outline).bg(colors.surface)
            } else {
                Style::default()
                    .fg(colors.outline_unfocused)
                    .bg(colors.surface)
            })
            .style(Style::default().fg(colors.on_surface).bg(colors.surface));
        if let Some(title) = title {
            block = block
                .title(title)
                .title_style(title_style)
                .title_alignment(Alignment::Left)
        }
        block
    })
}

pub fn tertiary_text() -> Style {
    with_theme(|t| {
        let colors = &t.colors;
        Style::default().fg(colors.on_surface).bg(colors.surface)
    })
}

pub fn themed_tabs<'a>(
    title: Option<&'a str>,
    titles: Vec<Line<'a>>,
    selected: usize,
    has_focus: bool,
) -> Tabs<'a> {
    with_theme(|theme| {
        Tabs::new(titles)
            .style(
                Style::default()
                    .fg(theme.colors.on_surface)
                    .bg(theme.colors.surface),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .block(themed_block(title, has_focus))
            .select(selected)
    })
}

pub fn themed_table<'a, R, C>(
    rows: R,
    widths: C,
    title: Option<&'a str>,
    has_focus: bool,
) -> Table<'a>
where
    R: IntoIterator,
    R::Item: Into<Row<'a>>,
    C: IntoIterator,
    C::Item: Into<Constraint>,
{
    with_theme(|theme| {
        let row_highlight_style = if has_focus {
            Style::default()
                .bg(theme.colors.surface_hl)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Table::new(rows, widths)
            .block(themed_block(title, has_focus))
            .column_spacing(2)
            .row_highlight_style(row_highlight_style)
    })
}

pub fn themed_button(label: &str, selected: bool) -> Paragraph<'_> {
    with_theme(|theme| {
        let style = if selected {
            Style::default()
                .fg(theme.colors.on_primary)
                .bg(theme.colors.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.colors.on_surface)
                .bg(theme.colors.surface)
        };

        Paragraph::new(label)
            .style(style)
            .alignment(Alignment::Center)
    })
}

pub fn themed_info_block(message: &str) -> Paragraph<'_> {
    with_theme(|theme| {
        let style = Style::default()
            .fg(theme.colors.secondary)
            .bg(theme.colors.surface)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC);

        Paragraph::new(
            Line::from(Span::styled(message.to_string(), style)).alignment(Alignment::Center),
        )
        .alignment(ratatui::layout::Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.outline))
                .style(Style::default().bg(theme.colors.surface)),
        )
    })
}

#[macro_export]
macro_rules! themed_line {
    ($text:expr) => {{
        $crate::ui::framework::theme::with_theme(|t| {
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
        $crate::ui::framework::theme::with_theme(|t| {
            ratatui::widgets::Row::new($text)
                .style(ratatui::style::Style::default().fg(t.colors.on_surface))
        })
    }};
}
