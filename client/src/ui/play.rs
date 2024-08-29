use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use self::{
    chat::draw_chat, editor::draw_editor, goal::draw_goal, join::draw_join, lobby::draw_lobby,
};
use crate::{
    app::App,
    constants::{CHAT_SIZE, EDITOR_HEIGHT, GOAL_HEIGHT, PLAY_SIDE_WIDTH, TERMINAL_WIDTH},
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
                Layout::vertical([Constraint::Min(0), Constraint::Length(CHAT_SIZE as u16)])
                    .split(horizontal[0]);

            draw_lobby(f, vertical[0], lobby);
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
        }
        // If we are not connected to a lobby, draw the join form.
        Connection::Join(ref join) => {
            draw_join(f, app, area, join);
        }
        Connection::Offline(_) => {}
    }
}
