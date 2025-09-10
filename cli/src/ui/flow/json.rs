use bytes::Bytes;
use cow_utils::CowUtils;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;
use tracing::error;

use crate::notify_error;

// HACK:: need to find away around static here
pub fn highlight_json(raw: Bytes) -> Vec<Line<'static>> {
    match serde_json::from_str::<Value>(&String::from_utf8_lossy(&raw)) {
        Ok(json) => {
            let mut lines: Vec<Line> = vec![];
            walk(&json, &mut lines, 0);
            lines
        }
        Err(err) => {
            notify_error!("Json {}", err);
            let s = String::from_utf8_lossy(&raw).to_string();
            error!("Invalid json {}\n{}", err, s);
            s.cow_replace(":", ":\n")
                .lines()
                .map(|l| Line::from(l.to_string()))
                .collect()
        }
    }
}

fn walk(v: &Value, lines: &mut Vec<Line>, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match v {
        Value::Null => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled("null", Style::default().fg(Color::DarkGray)),
            ]));
        }

        Value::Bool(val) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(val.to_string(), Style::default().fg(Color::Magenta)),
            ]));
        }

        Value::Number(number) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(number.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }

        Value::String(s) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(format!("\"{s}\""), Style::default().fg(Color::Green)),
            ]));
        }

        Value::Array(values) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str.clone()),
                Span::styled("[", Style::default().fg(Color::DarkGray)),
            ]));
            for v in values {
                walk(v, lines, indent + 1);
            }
            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled("]", Style::default().fg(Color::DarkGray)),
            ]));
        }

        Value::Object(map) => {
            lines.push(Line::from(vec![
                Span::raw(indent_str.clone()),
                Span::styled("{", Style::default().fg(Color::DarkGray)),
            ]));

            for (key, value) in map {
                let mut spans = vec![
                    Span::raw("  ".repeat(indent + 1)),
                    Span::styled(format!("\"{key}\""), Style::default().fg(Color::Cyan)),
                    Span::raw(": "),
                ];

                match value {
                    Value::Null => {
                        spans.push(Span::styled("null", Style::default().fg(Color::DarkGray)));
                    }
                    Value::Bool(val) => {
                        spans.push(Span::styled(
                            val.to_string(),
                            Style::default().fg(Color::Magenta),
                        ));
                    }
                    Value::Number(num) => {
                        spans.push(Span::styled(
                            num.to_string(),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    Value::String(s) => {
                        spans.push(Span::styled(
                            format!("\"{s}\""),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    Value::Array(_) | Value::Object(_) => {
                        // Complex values: start new line and recurse
                        lines.push(Line::from(spans));
                        walk(value, lines, indent + 2);
                        continue;
                    }
                }

                lines.push(Line::from(spans));
            }

            lines.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled("}", Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
}
