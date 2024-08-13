use ratatui::{
    layout::Rect,
    terminal::Frame,
    widgets::{Block, List},
};

use crate::{
    schema::{encryption::Encryption, lobby::Lobby},
    ui::get_random_symbol,
};

pub fn draw_lobby(f: &mut Frame, area: Rect, lobby: &Lobby) {
    let title = if let Some(ref name) = lobby.name {
        name.as_str()
    } else {
        "Lobby"
    };
    let block = Block::bordered().title(title);
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
    f.render_widget(players, area);
}
