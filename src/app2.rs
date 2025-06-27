use std::collections::{HashMap, hash_map};

/// A Ratatui example that demonstrates how to create an interactive table with a scrollbar.
///
/// This example runs with the Ratatui library code in the branch that you are currently
/// reading. See the [`latest`] branch for the code which works with the most recent Ratatui
/// release.
///
/// [`latest`]: https://github.com/ratatui/ratatui/tree/latest
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{self, Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, BorderType, Cell, Clear, HighlightSpacing, List, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{DefaultTerminal, Frame};
use style::palette::tailwind;
use tracing::debug;

use crate::event::{AppEvent, Event, EventHandler, RequestData, ResponseData};

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 2] = [
    "(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right",
    "(Shift + →) next color | (Shift + ←) previous color",
];

const ITEM_HEIGHT: usize = 4;

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

pub struct App {
    running: bool,
    state: TableState,
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    requests: Vec<RequestData>,
    responses: HashMap<i64, ResponseData>,
    pub events: EventHandler,
    show_popup: bool,
    popup_scroll_state: ScrollbarState,
    popup_vertical_scroll: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            requests: vec![],
            responses: hash_map::HashMap::new(),
            events: EventHandler::new(),
            show_popup: false,
            popup_scroll_state: ScrollbarState::new(0),
            popup_vertical_scroll: 0,
        }
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.requests.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.requests.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub const fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub const fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub const fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    // if let Some(key) = event::read()?.as_key_press_event() {
                    if let Some(key) = event.as_key_press_event() {
                        let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
                        match key.code {
                            // KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('q') | KeyCode::Esc => {
                                if self.show_popup {
                                    self.show_popup = false
                                } else {
                                    self.quit()
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                if self.show_popup {
                                    self.popup_scroll_state.next();
                                } else {
                                    self.next_row()
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if self.show_popup {
                                    self.popup_scroll_state.prev();
                                } else {
                                    self.previous_row()
                                }
                            }
                            KeyCode::Char('l') | KeyCode::Right if shift_pressed => {
                                self.next_color()
                            }
                            KeyCode::Char('h') | KeyCode::Left if shift_pressed => {
                                self.previous_color();
                            }
                            KeyCode::Char('l') | KeyCode::Right => self.next_column(),
                            KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
                            KeyCode::Enter => self.view_req(),
                            _ => {}
                        }
                    }
                }
                Event::App(app_event) => match app_event {
                    AppEvent::Request(data) => self.requests.push(data),
                    AppEvent::Response(data) => {
                        let _ = self.responses.insert(data.id, data);
                    }
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        self.set_colors();

        self.render_table(frame, rects[0]);
        self.render_scrollbar(frame, rects[0]);
        self.render_footer(frame, rects[1]);
        self.render_popup(frame);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);

        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let rows = self
            .requests
            .iter()
            .map(|r| {
                let response = self.responses.get(&r.id);
                let c = match response {
                    Some(_) => "+",
                    None => "-",
                };
                format!("{} {}", c, r.request_line.as_str())
            })
            .map(|c| Row::new(vec![c]));

        let bar = " █ ";
        let t = Table::new(
            rows,
            [
                // + 1 is for padding.
                // Constraint::Length(self.longest_item_lens.0 + 1),
                // Constraint::Min(self.longest_item_lens.1 + 1),
                // Constraint::Min(self.longest_item_lens.2),
                Constraint::Fill(1),
            ],
        )
        // .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(self.colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(t, area, &mut self.state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        frame.render_widget(info_footer, area);
    }

    fn render_popup(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [instructions, content] = vertical.areas(area);

        // frame.render_widget(
        //     Line::from("Press 'p' to toggle popup, 'q' to quit").centered(),
        //     instructions,
        // );
        //
        // frame.render_widget(Block::bordered().title("Content").on_blue(), content);

        if self.show_popup {
            let idx = match self.state.selected() {
                Some(idx) => idx,
                None => {
                    self.show_popup = false;
                    return;
                }
            };

            let request = &self.requests[idx];
            let response = &self.responses.get(&request.id);

            let mut text = vec![
                Line::from("REQUEST"),
                Line::from(request.request_line.clone()),
            ];
            for header in request.headers.iter() {
                text.push(Line::from(header.to_string()));
            }
            if let Some(resp_data) = response {
                text.push(Line::from("".to_string()));
                text.push(Line::from("".to_string()));

                text.push(Line::from("RESPONSE".to_string()));
                if let Some(body) = &resp_data.body {
                    body.split("\n\r").for_each(|l| {
                        l.split("\r").for_each(|l2| {
                            l2.split("\n").for_each(|l3| {
                                text.push(Line::from(l3));
                            });
                        });
                    });
                };
            };

            self.popup_scroll_state = self.popup_scroll_state.content_length(text.len());
            let create_block = |title: &'static str| Block::bordered().gray().title(title.bold());

            let popup = Block::bordered().title("Popup");
            let paragraph = Paragraph::new(text.clone())
                .gray()
                // .block(create_block("Vertical scrollbar with arrows"))
                .block(popup)
                .scroll((self.popup_scroll_state.get_position() as u16, 0));

            // paragraph.block(popup);
            // let mut items = vec![request.request_line.clone(), request.headers.clone()];
            //
            // if let Some(resp_data) = response {
            //     items.push("\n\n".to_string());
            //     items.push(resp_data.request_line.clone());
            //     items.push(resp_data.headers.clone());
            //     if let Some(body) = &resp_data.body {
            //         items.push(body.clone());
            //     };
            // };
            // let list = List::new(items).block(popup).scroll_padding(1);

            let popup_area = self.centered_area(area, 60, 60);
            // clears out any background in the area before rendering the popup
            frame.render_widget(Clear, popup_area);
            frame.render_widget(paragraph, popup_area);
        }
    }

    /// Create a centered rect using up certain percentage of the available rect
    fn centered_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    fn view_req(&mut self) {
        self.show_popup = true;
    }
}
