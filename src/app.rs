use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use color_eyre::Result;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::prelude::{Frame, Style, Stylize};
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState, Tabs,
};
use ratatui::{DefaultTerminal, style};
use style::palette::tailwind;

use crate::event::{AppEvent, Event, EventHandler};
use crate::flow::FlowStore;
use crate::ui::key_popup::KeyHelpPopup;
use crate::ui::log::render_log_popup;
use crate::ui::splash;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
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
    pub popup: Option<FlowPopup>,
}

struct UiFlow {
    pub line: String,
}

struct FlowPopup {
    request: Option<Vec<String>>,
    response: Option<Vec<String>>,
    certs: Option<Vec<String>>,
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
    popup_tab_index: usize,
    key_help: KeyHelpPopup,
    key_popup: bool,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
    log_popup: bool,
    log_scroll: usize,
}

impl App {
    pub fn new(flow_store: FlowStore, log_buffer: Arc<Mutex<VecDeque<String>>>) -> Self {
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
            popup_tab_index: 0,
            key_help: KeyHelpPopup::default(),
            key_popup: false,
            log_buffer,
            log_popup: false,
            log_scroll: 0,
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
                if let Some(entry) = self.flow_store.flows.get(id) {
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
                        .map(|r| r.line_pretty())
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
                        let mut req = vec![];
                        if let Some(request) = &flow.request {
                            req.push(request.line_pretty());
                            for (k, v) in request.headers.iter() {
                                req.push(format!("{}: {}", k, v));
                            }
                            if let Some(body) = &request.body {
                                req.push(body.clone());
                            }
                        }
                        let mut resp = vec![];
                        if let Some(response) = &flow.response {
                            resp.push(response.request_line());
                            for (k, v) in response.headers.iter() {
                                resp.push(format!("{}: {}", k, v));
                            }
                            if let Some(body) = &response.body {
                                resp.push(body.clone());
                            }
                        }

                        let mut c = vec![];
                        if let Some(certs) = &flow.cert_info {
                            for cert in certs.iter() {
                                c.push(format!("  - {}", cert.issuer));
                            }
                        }
                        Some(FlowPopup {
                            request: Some(req),
                            response: Some(resp),
                            certs: Some(c),
                        })
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
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                if self.show_popup {
                                    self.popup_scroll_state.first();
                                    self.show_popup = false
                                } else if self.key_popup {
                                    self.key_popup = false
                                } else if self.log_popup {
                                    self.log_popup = false;
                                } else {
                                    self.quit()
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                if self.show_popup {
                                    self.popup_scroll_state.next();
                                } else if self.log_popup {
                                    self.log_scroll += 1;
                                } else {
                                    self.next_row()
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if self.show_popup {
                                    self.popup_scroll_state.prev();
                                } else if self.log_popup {
                                    if self.log_scroll > 0 {
                                        self.log_scroll -= 1;
                                    }
                                } else {
                                    self.previous_row()
                                }
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                if self.show_popup {
                                    self.popup_tab_index = (self.popup_tab_index + 1) % 3;
                                    self.popup_scroll_state.first();
                                } else {
                                    self.next_color();
                                }
                            }
                            KeyCode::Char('h') | KeyCode::Left => {
                                if self.show_popup {
                                    self.popup_tab_index = (self.popup_tab_index + 2) % 3; // HACK:
                                    // ugh yeah this works
                                    self.popup_scroll_state.first();
                                } else {
                                    self.previous_color();
                                }
                            }
                            KeyCode::Char('?') => self.key_popup = true,
                            KeyCode::Char('i') => self.log_popup = true,
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
        self.set_colors();

        if ui_state.flows.is_empty() {
            splash::render_splash(frame);
        } else {
            self.render_table(frame, frame.area(), &ui_state);
        }

        self.render_scrollbar(frame, frame.area());
        if self.key_popup {
            self.key_help.render(frame, frame.area());
        }

        if self.log_popup {
            render_log_popup(frame, frame.area(), &self.log_buffer, self.log_scroll);
        }
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

    fn render_popup(&mut self, frame: &mut Frame, popup: &Option<FlowPopup>) {
        let popup = match popup {
            Some(p) => p,
            None => {
                self.show_popup = false;
                return;
            }
        };

        let tab_titles = ["Request", "Response", "Certs"];
        let tabs = Tabs::new(
            tab_titles
                .iter()
                .cloned()
                .map(Line::from)
                .collect::<Vec<Line>>(),
        )
        .highlight_style(Style::default().bold())
        .bg(self.colors.buffer_bg)
        .fg(self.colors.row_fg)
        .select(self.popup_tab_index);

        // Select the correct content based on active tab
        let lines = match self.popup_tab_index {
            0 => popup.request.as_ref(),
            1 => popup.response.as_ref(),
            2 => popup.certs.as_ref(),
            _ => None,
        }
        .cloned()
        .unwrap_or_else(|| vec!["(empty)".to_string()])
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();

        self.popup_scroll_state = self.popup_scroll_state.content_length(lines.len());

        let paragraph = Paragraph::new(lines)
            .bg(self.colors.buffer_bg)
            .fg(self.colors.row_fg)
            .scroll((self.popup_scroll_state.get_position() as u16, 0));

        let area = frame.area();
        let popup_area = self.centered_area(area, 60, 60);
        let inner_area = Block::default()
            .borders(Borders::ALL)
            .title("Flow Info")
            .bg(self.colors.buffer_bg)
            .fg(self.colors.row_fg)
            .border_style(Style::default().fg(self.colors.footer_border_color))
            .inner(popup_area);

        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(inner_area);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(
            Block::default().borders(Borders::ALL).title("Flow Info"),
            popup_area,
        );
        frame.render_widget(tabs, layout[0]);
        frame.render_widget(paragraph, layout[1]);
    }

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

    pub fn tick(&self) {}

    fn view_req(&mut self) {
        self.show_popup = true;
    }
}
