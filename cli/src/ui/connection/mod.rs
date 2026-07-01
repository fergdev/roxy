use crossterm::event::MouseEvent;
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, ratatui::layout::Rect};
use ratatui::{
    text::Line,
    widgets::{Clear, Paragraph},
};
use roxy_proxy::flow_store::{FlowStore, ProxyConnection};
use tokio::{sync::watch, task::JoinHandle};
use tracing::error;

use crate::{
    action::Action,
    ui::framework::{
        component::{Component, DispatchError, DispatchResult},
        scroll::TwoAxisScrollState,
        theme::themed_block,
        util::centered_rect,
    },
};

pub struct ConnectionsComponent {
    area: Rect,
    focus: FocusFlag,

    ui_rx: watch::Receiver<Vec<String>>,
    handle: JoinHandle<()>,

    scroll: TwoAxisScrollState,
}

impl ConnectionsComponent {
    pub fn new(flow_store: FlowStore) -> Self {
        let data: Vec<String> = vec![];
        let (ui_tx, ui_rx) = watch::channel(data);
        let task_flow_store = flow_store.clone();
        let handle = tokio::spawn(async move {
            let mut connections_sub = task_flow_store.connections_notifier.subscribe();
            loop {
                while connections_sub.changed().await.is_ok() {
                    let mut connections: Vec<String> = vec![];
                    for id in flow_store.connections_ordered_ids.read().await.iter() {
                        if let Some(con) = flow_store.connections.get(id) {
                            let guard = con.as_ref().read().await;
                            connections.push(render_connection(&guard));
                        } else {
                            error!("Error getting connection");
                        }
                    }
                    ui_tx.send(connections).unwrap_or_else(|e| {
                        tracing::error!("Failed to send UI state update: {}", e);
                    });
                }
            }
        });
        Self {
            area: Rect::default(),
            focus: FocusFlag::new().with_name("ConnectionsComponent"),
            ui_rx,
            handle,
            scroll: TwoAxisScrollState::default(),
        }
    }
}

impl Drop for ConnectionsComponent {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl Component for ConnectionsComponent {
    fn area(&self) -> Rect {
        self.area
    }

    fn focus(&mut self) -> &mut FocusFlag {
        &mut self.focus
    }

    fn handle_mouse_event(&mut self, mouse: &MouseEvent) -> DispatchResult {
        self.scroll.handle_mouse_event(mouse);
        DispatchError::action(Action::FocusReq(self.focus.widget_id()))
    }

    fn handle_action(&mut self, action: &Action) -> DispatchResult {
        if self.scroll.handle_action(action) {
            DispatchError::stop()
        } else {
            Ok(())
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let popup_area = centered_rect(80, 60, area);
        self.area = popup_area;
        frame.render_widget(Clear, popup_area);
        let lines = self
            .ui_rx
            .borrow()
            .iter()
            .map(|connection| Line::from(connection.clone()))
            .collect::<Vec<_>>();
        let height = lines.len();
        let width = lines.iter().map(|line| line.width()).max().unwrap_or(0);

        self.scroll.set_content_height(height as u16);
        self.scroll.set_content_width(width as u16);

        frame.render_widget(
            Paragraph::new(lines)
                .block(themed_block(Some("Connections"), self.focus.get()))
                .scroll(self.scroll.offset()),
            popup_area,
        );
        self.scroll.render(frame, popup_area);
    }
}

fn render_connection(connection: &ProxyConnection) -> String {
    format!(
        "Msg: {:?}, Address: {}, TimeStamp: {:?}",
        connection.message, connection.socket_addr, connection.time_stamp
    )
}

impl HasFocus for ConnectionsComponent {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}
