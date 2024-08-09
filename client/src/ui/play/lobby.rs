use ratatui::{
    layout::Rect,
    terminal::Frame,
    widgets::{Block, List},
};

use crate::{
    schema::{lobby::Lobby, Encryption},
    ui::get_random_symbol,
};

pub fn draw_lobby(f: &mut Frame, area: Rect, lobby: &Lobby) {
    let block = Block::bordered().title("Lobby");
    let encrypted_names = lobby.encryptions.values().map(
        |Encryption {
             id: _,
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
