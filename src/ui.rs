use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Stylize},
    text::Text,
    widgets::{Block, BorderType, Paragraph, Row, Table, Widget},
};

use crate::app::App;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block2 = Block::bordered()
            .title("Roxy")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        if self.requests.is_empty() {
            let reqs_p = Paragraph::new("wow nothing here")
                .block(block2)
                .fg(Color::Cyan)
                .bg(Color::Black);

            reqs_p.render(area, buf);
        } else {
            let bar = " █ ";
            let request_cells = self
                .requests
                .iter()
                // .map(|r| Cell::from(Text::from(r.request_line.as_str())))
                // .map(|r| Cell::new(r.request_line.as_str()))
                .map(|r| r.request_line.as_str())
                .map(|c| Row::new(vec![c]));
            // .collect::<Vec<_>>();
            let t = Table::new(request_cells, [Constraint::Fill(1)]).highlight_symbol(Text::from(
                vec!["".into(), bar.into(), bar.into(), "".into()],
            ));
            t.render(area, buf);
        }
    }
}
