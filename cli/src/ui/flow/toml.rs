use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use toml::{Table as TomlTable, Value as TomlValue};

pub fn highlight_toml(src: &[u8]) -> Vec<Line<'static>> {
    let s = String::from_utf8_lossy(src);
    let mut lines = Vec::new();

    match s.parse::<TomlTable>() {
        Ok(root) => {
            walk_toml_table(&root, &mut lines, 0);
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

fn walk_toml_table(table: &TomlTable, lines: &mut Vec<Line<'static>>, indent: usize) {
    for (key, val) in table.iter() {
        match val {
            TomlValue::Table(tbl) => {
                lines.push(Line::from(vec![
                    Span::raw("  ".repeat(indent)),
                    Span::styled("[", Style::default().fg(Color::DarkGray)),
                    Span::styled(key.to_string(), Style::default().fg(Color::Blue)),
                    Span::styled("]", Style::default().fg(Color::DarkGray)),
                ]));
                walk_toml_table(tbl, lines, indent + 1);
            }

            TomlValue::Array(arr) if is_array_of_tables(arr) => {
                for tbl in arr.iter().filter_map(|v| v.as_table()) {
                    lines.push(Line::from(vec![
                        Span::raw("  ".repeat(indent)),
                        Span::styled("[[", Style::default().fg(Color::DarkGray)),
                        Span::styled(key.to_string(), Style::default().fg(Color::Blue)),
                        Span::styled("]]", Style::default().fg(Color::DarkGray)),
                    ]));
                    walk_toml_table(tbl, lines, indent + 1);
                }
            }

            _ => {
                lines.push(toml_value_line(key, val, indent));
            }
        }
    }
}

fn toml_value_line(key: &str, val: &TomlValue, indent: usize) -> Line<'static> {
    let value_span = match val {
        TomlValue::String(s) => Span::styled(format!("{s:?}"), Style::default().fg(Color::Green)),
        TomlValue::Integer(i) => Span::styled(i.to_string(), Style::default().fg(Color::Magenta)),
        TomlValue::Float(f) => Span::styled(f.to_string(), Style::default().fg(Color::Magenta)),
        TomlValue::Boolean(b) => Span::styled(b.to_string(), Style::default().fg(Color::Yellow)),
        TomlValue::Datetime(dt) => Span::styled(dt.to_string(), Style::default().fg(Color::Cyan)),
        TomlValue::Array(arr) => highlight_toml_array(arr),
        TomlValue::Table(tbl) => {
            let preview = inline_table_preview(tbl);
            Span::styled(preview, Style::default().fg(Color::White))
        }
    };

    Line::from(vec![
        Span::raw("  ".repeat(indent)),
        Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(" = ", Style::default().fg(Color::DarkGray)),
        value_span,
    ])
}

fn highlight_toml_array(arr: &[TomlValue]) -> Span<'static> {
    let content = arr
        .iter()
        .map(|v| match v {
            TomlValue::String(s) => format!("{s:?}"),
            TomlValue::Integer(i) => i.to_string(),
            TomlValue::Float(f) => f.to_string(),
            TomlValue::Boolean(b) => b.to_string(),
            TomlValue::Datetime(dt) => dt.to_string(),
            TomlValue::Array(inner) => format!("[{}]", inner.len()),
            TomlValue::Table(tbl) => inline_table_preview(tbl),
        })
        .collect::<Vec<_>>()
        .join(", ");

    Span::styled(format!("[{content}]"), Style::default().fg(Color::White))
}

fn inline_table_preview(tbl: &TomlTable) -> String {
    let mut parts = Vec::with_capacity(tbl.len());
    for (k, v) in tbl.iter() {
        let val = match v {
            TomlValue::String(s) => format!("{s:?}"),
            TomlValue::Integer(i) => i.to_string(),
            TomlValue::Float(f) => f.to_string(),
            TomlValue::Boolean(b) => b.to_string(),
            TomlValue::Datetime(dt) => dt.to_string(),
            TomlValue::Array(a) => format!("[{}]", a.len()),
            TomlValue::Table(_) => "{…}".to_string(),
        };
        parts.push(format!("{k} = {val}"));
    }
    format!("{{ {} }}", parts.join(", "))
}

fn is_array_of_tables(arr: &[TomlValue]) -> bool {
    !arr.is_empty() && arr.iter().all(|v| matches!(v, TomlValue::Table(_)))
}
