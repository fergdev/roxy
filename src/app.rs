use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{self, Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, BorderType, Clear, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState,
};
use ratatui::{DefaultTerminal, Frame};
use style::palette::tailwind;

use crate::event::{AppEvent, Event, EventHandler};
use crate::flow::FlowStore;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 2] = [
    "(q) quit | (k) move up | (j) move down ",
    "(L) next color | (H) previous color",
];

const ITEM_HEIGHT: usize = 4;

struct TableColors {
    buffer_bg: Color,
    // header_bg: Color,
    // header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    // normal_row_color: Color,
    // alt_row_color: Color,
    footer_border_color: Color,
}

struct UiState {
    pub flows: Vec<UiFlow>,
    pub popup: Option<Vec<String>>,
}

struct UiFlow {
    pub line: String,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            // header_bg: color.c900,
            // header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            // normal_row_color: tailwind::SLATE.c950,
            // alt_row_color: tailwind::SLATE.c900,
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
    flow_store: FlowStore,
    events: EventHandler,
    show_popup: bool,
    popup_scroll_state: ScrollbarState,
}

impl App {
    pub fn new(flow_store: FlowStore) -> Self {
        Self {
            running: true,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            flow_store,
            events: EventHandler::new(),
            show_popup: false,
            popup_scroll_state: ScrollbarState::new(0),
        }
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let len = self.flow_store.flows.len();
                if i >= len - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let len = self.flow_store.flows.len();
                if i == 0 { len - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
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
            let fs = self.flow_store.clone();
            let ids = fs.ordered_ids.read().await;

            let mut rows = Vec::new();
            for id in ids.iter() {
                if let Some(entry) = self.flow_store.flows.get(&id) {
                    let flow = entry.value().read().await;
                    let c = if flow.response.is_some() { "+" } else { "-" };
                    let resp = flow
                        .response
                        .as_ref()
                        .map(|r| r.status.to_string())
                        .unwrap_or_else(|| "-".into());
                    let req = flow
                        .request
                        .as_ref()
                        .map(|r| r.request_line())
                        .unwrap_or_else(|| "empty".into());

                    rows.push(UiFlow {
                        line: format!("{} {} -> {}", c, req, resp),
                    });
                }
            }

            let popup = if self.show_popup {
                match self.state.selected() {
                    Some(i) => {
                        if i > ids.len() {
                            self.show_popup = false;
                            continue;
                        }
                        let id = ids[i];
                        let entry = self.flow_store.flows.get(&id).unwrap();
                        let flow = entry.value().read().await;
                        let mut text = vec![];
                        if let Some(request) = &flow.request {
                            text.push(request.request_line());
                            for (k, v) in request.headers.iter() {
                                text.push(format!("{}: {}", k, v));
                            }
                            if let Some(body) = &request.body {
                                text.push(body.clone());
                            }
                        }
                        if let Some(response) = &flow.response {
                            text.push(response.request_line());
                            for (k, v) in response.headers.iter() {
                                text.push(format!("{}: {}", k, v));
                            }
                            if let Some(body) = &response.body {
                                text.push(body.clone());
                            }
                        }

                        if let Some(certs) = &flow.cert_info {
                            text.push("Certificates:".into());
                            for cert in certs.iter() {
                                text.push(format!("  - {}", cert.issuer));
                            }
                        }
                        Some(text)
                    }

                    None => None,
                }
            } else {
                None
            };

            let ui_state = UiState { flows: rows, popup };

            terminal.draw(|frame| self.render(frame, ui_state))?;

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    if let Some(key) = event.as_key_press_event() {
                        let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                if self.show_popup {
                                    self.popup_scroll_state.first();
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
                            KeyCode::Char('L') | KeyCode::Right if shift_pressed => {
                                self.next_color()
                            }
                            KeyCode::Char('H') | KeyCode::Left if shift_pressed => {
                                self.previous_color();
                            }
                            KeyCode::Enter => self.view_req(),
                            _ => {}
                        }
                    }
                }
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, ui_state: UiState) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        self.set_colors();

        self.render_table(frame, rects[0], &ui_state);
        self.render_scrollbar(frame, rects[0]);
        self.render_footer(frame, rects[1]);
        self.render_popup(frame, &ui_state.popup);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, ui_state: &UiState) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);

        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let rows = ui_state
            .flows
            .iter()
            .map(|f| Row::new(vec![f.line.clone()]));
        let bar = " █ ";

        let t = Table::new(rows, [Constraint::Fill(1)])
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

    fn render_popup(&mut self, frame: &mut Frame, popup: &Option<Vec<String>>) {
        let popup = match popup {
            Some(p) => p,
            None => {
                self.show_popup = false;
                return;
            }
        };

        let area = frame.area();

        let mut text = vec![];
        popup.iter().for_each(|line| {
            text.push(Line::from(line.clone()));
        });

        self.popup_scroll_state = self.popup_scroll_state.content_length(text.len());

        let popup = Block::bordered().title("Info");
        let paragraph = Paragraph::new(text.clone())
            .gray()
            .block(popup)
            .scroll((self.popup_scroll_state.get_position() as u16, 0));

        let popup_area = self.centered_area(area, 60, 60);
        // clears out any background in the area before rendering the popup
        frame.render_widget(Clear, popup_area);
        frame.render_widget(paragraph, popup_area);
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
