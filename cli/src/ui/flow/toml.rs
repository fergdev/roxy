use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

pub fn highlight_toml(src: &[u8]) -> Vec<Line<'static>> {
    let str = String::from_utf8_lossy(src); // TODO: parse with reader
    let parsed = str.parse::<DocumentMut>();
    let mut lines = Vec::new();

    match parsed {
        Ok(doc) => {
            walk_toml_table(&doc, &mut lines, 0);
        }
        Err(e) => {
            lines.push(Line::from(vec![Span::styled(
                format!("Invalid TOML: {e}"),
                Style::default().fg(Color::Red),
            )]));
        }
    }

    lines
}

fn walk_toml_table(table: &Table, lines: &mut Vec<Line<'static>>, indent: usize) {
    for (key, item) in table.iter() {
        match item {
            Item::Value(val) => {
                lines.push(toml_value_line(key, val, indent));
            }
            Item::Table(tbl) => {
                lines.push(Line::from(vec![
                    Span::raw("  ".repeat(indent)),
                    Span::styled("[", Style::default().fg(Color::DarkGray)),
                    Span::styled(key.to_string(), Style::default().fg(Color::Blue)),
                    Span::styled("]", Style::default().fg(Color::DarkGray)),
                ]));
                walk_toml_table(tbl, lines, indent);
            }
            Item::ArrayOfTables(arr) => {
                for t in arr.iter() {
                    lines.push(Line::from(vec![
                        Span::raw("  ".repeat(indent)),
                        Span::styled("[[", Style::default().fg(Color::DarkGray)),
                        Span::styled(key.to_string(), Style::default().fg(Color::Blue)),
                        Span::styled("]]", Style::default().fg(Color::DarkGray)),
                    ]));
                    walk_toml_table(t, lines, indent);
                }
            }
            _ => {}
        }
    }
}

fn toml_value_line(key: &str, val: &Value, indent: usize) -> Line<'static> {
    let value_span = match val {
        Value::String(s) => Span::styled(
            format!("\"{}\"", s.value()),
            Style::default().fg(Color::Green),
        ),
        Value::Integer(i) => Span::styled(i.to_string(), Style::default().fg(Color::Magenta)),
        Value::Float(f) => Span::styled(f.to_string(), Style::default().fg(Color::Magenta)),
        Value::Boolean(b) => Span::styled(b.to_string(), Style::default().fg(Color::Yellow)),
        Value::Datetime(dt) => Span::styled(dt.to_string(), Style::default().fg(Color::Cyan)),
        Value::Array(arr) => highlight_toml_array(arr),
        _ => Span::raw("<unknown>".to_string()),
    };

    Line::from(vec![
        Span::raw("  ".repeat(indent)),
        Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(" = ", Style::default().fg(Color::DarkGray)),
        value_span,
    ])
}

fn highlight_toml_array(arr: &Array) -> Span<'static> {
    let content = arr
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Span::styled(format!("[{content}]"), Style::default().fg(Color::White))
}
