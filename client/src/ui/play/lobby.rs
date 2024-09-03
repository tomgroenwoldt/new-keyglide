use chrono::Utc;
use common::constants::MAX_LOBBY_SIZE;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, List},
    Frame,
};

use crate::{
    app::App,
    schema::{encryption::Encryption, lobby::Lobby},
    ui::get_random_symbol,
};

pub fn draw_lobby(f: &mut Frame, app: &App, area: Rect, lobby: &Lobby) {
    let chunks = Layout::vertical([
        Constraint::Length(MAX_LOBBY_SIZE as u16 + 2),
        Constraint::Min(0),
    ])
    .split(area);

    let time = match lobby.status {
        common::LobbyStatus::WaitingForPlayers => None,
        common::LobbyStatus::AboutToStart(time) => Some(time),
        common::LobbyStatus::InProgress(time) => Some(time),
        common::LobbyStatus::Finish(time) => Some(time),
    };

    let title = lobby.name.as_str();
    let mut block = Block::bordered()
        .title(title)
        .title_bottom(lobby.status.to_string());

    if let Some(time) = time {
        let now = Utc::now();
        let remaining_millis = time.signed_duration_since(now).num_milliseconds();
        let seconds_with_millis = remaining_millis as f64 / 1000.0;
        let text = format!("{:.1}s", seconds_with_millis);
        block = block.title_bottom(Line::from(text).right_aligned());
    }

    let encrypted_names = lobby.encryptions.values().map(
        |Encryption {
             action: _,
             index,
             value,
         }| {
            value
                .chars()
                .enumerate()
                .map(|(i, c)| if i < *index { c } else { get_random_symbol() })
                .collect::<String>()
        },
    );
    let players = List::new(encrypted_names).block(block);
    f.render_widget(players, chunks[0]);

    // Render a small help section for the lobby owner.
    if lobby.local_player == lobby.owner && lobby.local_player.is_some() {
        let commands = vec![format!(
            "{} - Start the lobby",
            app.config.key_bindings.lobby.start.to_string()
        )];
        let block = Block::bordered().title("Owner commands");
        let command_list = List::new(commands).block(block);
        f.render_widget(command_list, chunks[1]);
    }
}
