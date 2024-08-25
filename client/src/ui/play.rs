use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use common::constants::MAX_LOBBY_SIZE;
use common::LobbyStatus;

use self::{
    chat::draw_chat, editor::draw_editor, goal::draw_goal, join::draw_join, lobby::draw_lobby,
};
use super::centered_rect;
use crate::{
    app::App,
    constants::{EDITOR_HEIGHT, GOAL_HEIGHT, PLAY_SIDE_WIDTH, TERMINAL_WIDTH},
    schema::connection::Connection,
};

pub mod chat;
pub mod editor;
pub mod goal;
pub mod join;
mod lobby;

pub fn draw_play_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.connection {
        Connection::Lobby(ref lobby) => {
            let horizontal = Layout::horizontal([
                Constraint::Percentage((PLAY_SIDE_WIDTH * 100.0) as u16),
                Constraint::Percentage((TERMINAL_WIDTH * 100.0) as u16),
            ])
            .split(area);
            let vertical =
                Layout::vertical([Constraint::Max(MAX_LOBBY_SIZE as u16), Constraint::Min(0)])
                    .split(horizontal[0]);

            draw_lobby(f, app, vertical[0], lobby);
            draw_chat(f, app, vertical[1], lobby);

            let layout = Layout::new(
                lobby.terminal_layout_direction,
                [
                    // Convert constant heights into integer percentage values.
                    Constraint::Percentage((EDITOR_HEIGHT * 100.0) as u16),
                    Constraint::Percentage((GOAL_HEIGHT * 100.0) as u16),
                ],
            )
            .split(horizontal[1]);

            draw_editor(f, app, layout[0]);
            draw_goal(f, app, layout[1]);

            if let LobbyStatus::AboutToStart(start_date) = lobby.status {
                draw_start_timer(f, area, start_date);
            }
        }
        // If we are not connected to a lobby, draw the join form.
        Connection::Join(ref join) => {
            draw_join(f, app, area, join);
        }
        Connection::Offline(_) => {}
    }
}

fn draw_start_timer(f: &mut Frame, area: Rect, start_date: DateTime<Utc>) {
    let popup = Block::bordered()
        .title("Get ready")
        .border_style(Style::default().fg(Color::LightYellow));
    let now = Utc::now();
    let remaining_millis = start_date.signed_duration_since(now).num_milliseconds();
    let seconds_with_millis = remaining_millis as f64 / 1000.0;
    let text = format!("Game is starting in {:.1}s.", seconds_with_millis);

    let area = centered_rect(area, text.len() as u16, 1);
    let paragraph = Paragraph::new(text).block(popup);
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
