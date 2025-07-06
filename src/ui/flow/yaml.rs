use std::io::Cursor;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use serde_yaml::Value;

pub fn pretty_print_yaml(raw: &[u8]) -> Vec<Line<'static>> {
    let cursor = Cursor::new(raw);
    match serde_yaml::from_reader(cursor) {
        Ok(value) => {
            let mut lines = vec![];
            walk_yaml(&value, &mut lines, 0);
            lines
        }
        Err(_) => {
            vec![Line::raw("Failed to parse YAML")]
        }
    }
}

// TODO: we need to print out maps properly
fn walk_yaml(value: &Value, lines: &mut Vec<Line>, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match value {
        Value::Null => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled("null", Style::default().fg(Color::DarkGray)),
            ]));
        }
        Value::Bool(b) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(b.to_string(), Style::default().fg(Color::Magenta)),
            ]));
        }
        Value::Number(n) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(n.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }
        Value::String(s) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(format!("\"{}\"", s), Style::default().fg(Color::Green)),
            ]));
        }
        Value::Sequence(seq) => {
            for item in seq {
                let prefix = format!("{}- ", indent_str);

                // TODO: awful aswell
                match item {
                    Value::String(s) => lines.push(Line::from(vec![
                        Span::raw(prefix.clone()),
                        Span::styled(format!("{:?}", s), Style::default().fg(Color::Green)),
                    ])),
                    Value::Number(n) => lines.push(Line::from(vec![
                        Span::raw(prefix.clone()),
                        Span::styled(n.to_string(), Style::default().fg(Color::Yellow)),
                    ])),
                    Value::Bool(b) => lines.push(Line::from(vec![
                        Span::raw(prefix.clone()),
                        Span::styled(b.to_string(), Style::default().fg(Color::Magenta)),
                    ])),
                    Value::Null => lines.push(Line::from(vec![
                        Span::raw(prefix.clone()),
                        Span::styled("null", Style::default().fg(Color::DarkGray)),
                    ])),
                    _ => {
                        // Print just the dash for complex type
                        lines.push(Line::from(Span::raw(prefix)));
                        walk_yaml(item, lines, indent + 1);
                    }
                }
            }
        }
        // TODO: this is awful will parse properly soon
        Value::Mapping(map) => {
            for (k, v) in map {
                let key = match k {
                    Value::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };

                match v {
                    Value::String(s) => {
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(format!("{}: ", key), Style::default().fg(Color::Blue)),
                            Span::styled(format!("{:?}", s), Style::default().fg(Color::Green)),
                        ]));
                    }
                    Value::Number(n) => {
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(format!("{}: ", key), Style::default().fg(Color::Blue)),
                            Span::styled(n.to_string(), Style::default().fg(Color::Yellow)),
                        ]));
                    }
                    Value::Bool(b) => {
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(format!("{}: ", key), Style::default().fg(Color::Blue)),
                            Span::styled(b.to_string(), Style::default().fg(Color::Magenta)),
                        ]));
                    }
                    Value::Null => {
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(
                                format!("{}: null", key),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                    _ => {
                        // key first
                        lines.push(Line::from(vec![
                            Span::raw(indent_str.clone()),
                            Span::styled(format!("{}:", key), Style::default().fg(Color::Blue)),
                        ]));
                        walk_yaml(v, lines, indent + 1);
                    }
                }
            }
        }
        Value::Tagged(tagged_value) => {
            let tag = &tagged_value.tag;
            let inner = &tagged_value.value;

            lines.push(Line::from(vec![
                Span::raw("  ".repeat(indent)),
                Span::styled(format!("!{} ", tag), Style::default().fg(Color::Cyan)),
            ]));

            walk_yaml(inner, lines, indent + 1);
        }
    }
}
