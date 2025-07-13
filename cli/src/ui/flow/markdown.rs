use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_markdown(markdown: &[u8]) -> Vec<Line<'static>> {
    let data = String::from_utf8_lossy(markdown);
    let parser = Parser::new(&data);

    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut style = Style::default();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level,
                id,
                classes,
                attrs,
            }) => {
                style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);

                // Optional metadata preview
                let mut heading_meta = String::new();
                if let Some(id) = id {
                    heading_meta += &format!(" [id: {id}]");
                }
                if !classes.is_empty() {
                    heading_meta += &format!(" [class: {}]", classes.join(","));
                }
                if !attrs.is_empty() {
                    let formatted: Vec<String> = attrs
                        .iter()
                        .map(|(k, v)| match v {
                            Some(val) => format!("{k}={val}"),
                            None => k.to_string(),
                        })
                        .collect();

                    heading_meta += &format!(" [attrs: {}]", formatted.join(","));
                }

                current_line.push(Span::styled(
                    format!("{}{}", "#".repeat(level as usize), heading_meta),
                    style,
                ));
            }

            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    style = Style::default();
                }
                TagEnd::Paragraph => {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    lines.push(Line::from(""));
                }
                TagEnd::CodeBlock => {
                    lines.push(Line::from(vec![Span::raw("```")]));
                }
                _ => {
                    style = Style::default();
                }
            },

            Event::Text(text) => {
                let span = Span::styled(text.to_string(), style);
                current_line.push(span);
            }

            Event::Code(code) => {
                current_line.push(Span::styled(
                    format!("`{code}`"),
                    Style::default().fg(Color::LightBlue),
                ));
            }

            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }

            Event::Rule => {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked {
                    Span::styled("[x] ", Style::default().fg(Color::Green))
                } else {
                    Span::styled("[ ] ", Style::default().fg(Color::Red))
                };
                current_line.push(marker);
            }

            Event::InlineMath(expr) => {
                current_line.push(Span::styled(
                    format!("${expr}$"),
                    Style::default().fg(Color::Magenta),
                ));
            }

            Event::DisplayMath(expr) => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
                lines.push(Line::from(vec![Span::styled(
                    format!("$$ {expr} $$"),
                    Style::default().fg(Color::Magenta),
                )]));
            }

            Event::Html(html) | Event::InlineHtml(html) => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
                lines.push(Line::from(vec![Span::styled(
                    html.to_string(),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            Event::FootnoteReference(name) => {
                current_line.push(Span::styled(
                    format!("[^{name}]"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            _ => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}
