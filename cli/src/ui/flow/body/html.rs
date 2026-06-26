use kuchiki::{NodeData, NodeRef, parse_html, traits::TendrilSink};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub fn highlight_html_dom<'a, R: std::io::Read>(
    reader: &mut R,
) -> Result<Vec<Line<'a>>, std::io::Error> {
    let parser = parse_html().from_utf8();
    let dom = parser.read_from(reader)?;
    let mut out = Vec::new();
    walk_node(&dom, 0, &mut out);
    Ok(out)
}

fn walk_node(node: &NodeRef, depth: usize, out: &mut Vec<Line>) {
    match &node.data() {
        NodeData::Text(contents) => {
            let text = contents.borrow();
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(Line::from(Span::styled(
                    format!("{:indent$}{}", "", trimmed, indent = depth * 2),
                    Style::default().fg(Color::White),
                )));
            }
        }

        NodeData::Element(element_data) => {
            let mut spans = vec![];

            spans.push(Span::styled(
                format!("{:indent$}<", "", indent = depth * 2),
                Style::default().fg(Color::DarkGray),
            ));

            spans.push(Span::styled(
                element_data.name.local.to_string(),
                Style::default().fg(Color::Blue),
            ));

            for attr in element_data.attributes.borrow().map.iter() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    attr.0.local.to_string(),
                    Style::default().fg(Color::Cyan),
                ));
                spans.push(Span::styled("=\"", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    attr.1.value.to_string(),
                    Style::default().fg(Color::Green),
                ));
                spans.push(Span::styled("\"", Style::default().fg(Color::DarkGray)));
            }

            spans.push(Span::styled(">", Style::default().fg(Color::DarkGray)));
            out.push(Line::from(spans));
        }

        NodeData::Comment(contents) => {
            out.push(Line::from(Span::styled(
                format!(
                    "{:indent$}<!--{}-->",
                    "",
                    contents.borrow().clone(),
                    indent = depth * 2
                ),
                Style::default().fg(Color::Magenta),
            )));
        }

        _ => {}
    }

    for child in node.children() {
        walk_node(&child, depth + 1, out);
    }

    if let NodeData::Element(element_data) = &node.data() {
        out.push(Line::from(vec![
            Span::styled(
                format!("{:indent$}</", "", indent = depth * 2),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                element_data.name.local.to_string(),
                Style::default().fg(Color::Blue),
            ),
            Span::styled(">", Style::default().fg(Color::DarkGray)),
        ]));
    }
}
