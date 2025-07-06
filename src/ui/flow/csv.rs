use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::Cursor;

pub fn render_csv(data: &[u8]) -> Vec<Line<'static>> {
    let mut rdr = csv::Reader::from_reader(Cursor::new(data));
    render(&mut rdr)
}

pub fn render_tsv(data: &[u8]) -> Vec<Line<'static>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(Cursor::new(data));
    render(&mut rdr)
}

fn render(rdr: &mut csv::Reader<Cursor<&[u8]>>) -> Vec<Line<'static>> {
    let mut lines = vec![];
    let headers = rdr.headers().unwrap();

    // Render headers
    lines.push(Line::from(
        headers
            .iter()
            .map(|h| {
                Span::styled(
                    format!("{:^15}", h),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    ));

    // Render rows
    rdr.records().for_each(|result| {
        if let Ok(record) = result {
            lines.push(Line::from(
                record
                    .iter()
                    .map(|val| Span::raw(format!("{:^15}", val)))
                    .collect::<Vec<_>>(),
            ));
        }
    });

    lines
}
