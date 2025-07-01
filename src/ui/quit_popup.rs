use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{event::Action, tui::Event};

use super::{component::Component, util::centered_rect};

pub struct QuitPopup {
    options: Vec<&'static str>,
    selected: usize,
}

impl QuitPopup {
    pub fn new() -> Self {
        Self {
            options: vec!["No", "Yes"],
            selected: 0,
        }
    }
}

impl Default for QuitPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for QuitPopup {
    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
            _ => Ok(None),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected < self.options.len() - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.options[self.selected] == "Yes" {
                    return Ok(Some(Action::Quit));
                } else {
                    return Ok(None); // Let app close popup manually
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Ok(None)
            }
            Action::Right => {
                if self.selected < self.options.len() - 1 {
                    self.selected += 1;
                }
                Ok(None)
            }
            Action::Select => {
                if self.options[self.selected] == "Yes" {
                    Ok(Some(Action::Quit))
                } else {
                    Ok(Some(Action::Back))
                }
            }
            _ => Ok(None), // No other actions handled here
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let popup_area = centered_rect(80, 40, area);

        let title = Paragraph::new("Are you sure you want to quit?")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Confirm Exit"));

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(popup_area);

        let button_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);

        let yes = Paragraph::new("Yes")
            .style(if self.selected == 1 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        let no = Paragraph::new("No")
            .style(if self.selected == 0 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(Clear, popup_area); // Clear background

        f.render_widget(yes, button_layout[1]);
        f.render_widget(no, button_layout[0]);

        f.render_widget(title, layout[0]);
        Ok(())
    }
}
