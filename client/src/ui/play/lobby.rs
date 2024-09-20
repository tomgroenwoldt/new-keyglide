use chrono::Utc;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    text::Line,
    widgets::{Block, Gauge, List},
    Frame,
};

use crate::{
    config::Config,
    schema::{encryption::Encryption, lobby::Lobby},
    ui::get_random_symbol,
};

pub fn draw_lobby(f: &mut Frame, area: Rect, config: &Config, lobby: &mut Lobby) {
    let player_count = lobby.encryptions.len();
    let waiting_player_count = lobby.waiting_encryptions.len();
    let mut constraints = vec![
        Constraint::Length((player_count * 3) as u16 + 2),
        Constraint::Min(0),
    ];
    if waiting_player_count > 0 {
        constraints.push(Constraint::Length(waiting_player_count as u16 + 2));
    }
    let chunks = Layout::vertical(constraints).split(area);

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

    // We split the lobby section into multiple chunks. Each chunks holds
    // one player and is exactly three rows high. This is due to the gauge +
    // bordered block we render per player.
    let constraints = (0..player_count).map(|_| Constraint::Length(3));
    let inner_chunks = Layout::vertical(constraints).split(chunks[0].inner(Margin {
        vertical: 1,
        horizontal: 1,
    }));
    for (i, encryption) in lobby.encryptions.iter().enumerate() {
        let (
            player_id,
            Encryption {
                action: _,
                index,
                value,
            },
        ) = encryption;
        let encryption = value
            .chars()
            .enumerate()
            .map(|(i, c)| if i < *index { c } else { get_random_symbol() })
            .collect::<String>();
        let mut gauge = Gauge::default().block(Block::bordered().title(encryption));
        if let Some(player) = lobby.players.get(player_id) {
            gauge = gauge.ratio(player.progress);
        };
        f.render_widget(gauge, inner_chunks[i]);
    }
    f.render_widget(block, chunks[0]);

    draw_lobby_commands(f, config, chunks[1], lobby);

    if waiting_player_count > 0 {
        let encrypted_names = lobby.waiting_encryptions.values().map(
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
        let waiting_players =
            List::new(encrypted_names).block(Block::bordered().title("Waiting room"));
        f.render_widget(waiting_players, chunks[2]);
    }
}

fn draw_lobby_commands(f: &mut Frame, config: &Config, area: Rect, lobby: &Lobby) {
    let mut commands = vec![format!(
        "{} - Disconnect from the lobby",
        config.key_bindings.lobby.disconnect
    )];

    // Add lobby owner specific commands depending on the lobby status.
    if lobby.local_player == lobby.owner && lobby.local_player.is_some() {
        match lobby.status {
            common::LobbyStatus::WaitingForPlayers => {
                commands.push(format!(
                    "{} - Start the lobby",
                    config.key_bindings.lobby.start
                ));
            }
            common::LobbyStatus::AboutToStart(_) => {}
            common::LobbyStatus::InProgress(_) => {}
            common::LobbyStatus::Finish(_) => {}
        }
    }

    let block = Block::bordered().title("Lobby commands");
    let command_list = List::new(commands).block(block);
    f.render_widget(command_list, area);
}
