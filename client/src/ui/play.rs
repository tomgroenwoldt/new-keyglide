use ratatui::{
    layout::{Constraint, Layout, Rect},
    terminal::Frame,
};

use crate::{
    app::{App, Connection},
    constants::CHAT_SIZE,
};

use self::{chat::draw_chat, editor::draw_editor, join::draw_join, lobby::draw_lobby};

mod chat;
mod editor;
mod join;
mod lobby;

pub fn draw_play_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.connection {
        Connection::Lobby(ref lobby) => {
            let horizontal =
                Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)])
                    .split(area);
            let vertical =
                Layout::vertical([Constraint::Min(0), Constraint::Length(CHAT_SIZE as u16)])
                    .split(horizontal[0]);

            draw_lobby(f, vertical[0], lobby);
            draw_chat(f, app, vertical[1], lobby);

            draw_editor(f, app, horizontal[1]);
        }
        // If we are not connected to a lobby, draw the join form.
        Connection::Join(ref join) => {
            draw_join(f, app, area, join);
        }
        Connection::Offline => {}
    }
}
