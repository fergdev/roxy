use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct KeyHelpPopup {
    pub lines: Vec<String>,
}

impl Default for KeyHelpPopup {
    fn default() -> Self {
        Self {
            lines: vec![
                "(q) Quit".to_string(),
                "(j) Move Down".to_string(),
                "(k) Move Up".to_string(),
                "(l) Next Color".to_string(),
                "(h) Previous Color".to_string(),
                "(Enter) View Request".to_string(),
            ],
        }
    }
}

impl KeyHelpPopup {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let lines: Vec<Line> = self
            .lines
            .iter()
            .map(|l| Line::styled(l.clone(), Style::default().fg(Color::LightCyan)))
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Key Bindings")
            .border_style(Style::default().fg(Color::Magenta));

        let paragraph = Paragraph::new(lines).block(block);

        let popup_area = Self::centered_area(area, 40, self.lines.len() as u16 + 4);

        f.render_widget(Clear, popup_area); // clear behind
        f.render_widget(paragraph, popup_area);
    }

    fn centered_area(area: Rect, width: u16, height: u16) -> Rect {
        let popup_layout = Layout::vertical([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

        Layout::horizontal([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
    }
}
