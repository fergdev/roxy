use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use xmltree::Element;

pub fn pretty_print_xml(raw: &[u8]) -> Vec<Line<'static>> {
    match xmltree::Element::parse(raw) {
        Ok(elem) => {
            let mut out = Vec::new();
            walk_xml3(&elem, &mut out, 0);
            out
        }
        Err(_) => vec![Line::from("<invalid xml>")],
    }
}

// HACK: lots of cloning here to get around lifetimes
fn walk_xml3(elem: &Element, lines: &mut Vec<Line<'static>>, indent: usize) {
    let indent_str = "  ".repeat(indent);

    let mut line = vec![
        Span::raw(indent_str.clone()),
        Span::styled("<".to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(elem.name.clone(), Style::default().fg(Color::Blue)),
    ];

    for (k, v) in &elem.attributes {
        line.push(Span::raw(" ".to_string()));
        line.push(Span::styled(k.clone(), Style::default().fg(Color::Cyan)));
        line.push(Span::raw("=".to_string()));
        line.push(Span::styled(
            format!("\"{v}\""),
            Style::default().fg(Color::Green),
        ));
    }

    let has_elements = elem
        .children
        .iter()
        .any(|c| matches!(c, xmltree::XMLNode::Element(_)));

    let has_text = elem
        .get_text()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !has_elements && has_text {
        line.push(Span::styled(
            ">".to_string(),
            Style::default().fg(Color::DarkGray),
        ));

        if let Some(text) = elem.get_text() {
            let trimmed = text.trim().to_string();
            line.push(Span::styled(trimmed, Style::default().fg(Color::White)));
        }

        line.push(Span::styled(
            "</".to_string(),
            Style::default().fg(Color::DarkGray),
        ));
        line.push(Span::styled(
            elem.name.clone(),
            Style::default().fg(Color::Blue),
        ));
        line.push(Span::styled(
            ">".to_string(),
            Style::default().fg(Color::DarkGray),
        ));

        lines.push(Line::from(line));
    } else {
        line.push(Span::styled(
            ">".to_string(),
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::from(line));

        for child in &elem.children {
            match child {
                xmltree::XMLNode::Element(child_elem) => {
                    walk_xml3(child_elem, lines, indent + 1);
                }
                xmltree::XMLNode::Text(t) => {
                    let trimmed = t.trim();
                    if !trimmed.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("  ".repeat(indent + 1)),
                            Span::styled(trimmed.to_string(), Style::default().fg(Color::White)),
                        ]));
                    }
                }
                _ => {}
            }
        }

        lines.push(Line::from(vec![
            Span::raw(indent_str.clone()),
            Span::styled("</".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(elem.name.clone(), Style::default().fg(Color::Blue)),
            Span::styled(">".to_string(), Style::default().fg(Color::DarkGray)),
        ]));
    }
}
