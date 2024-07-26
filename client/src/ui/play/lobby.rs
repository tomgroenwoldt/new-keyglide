use rand::{thread_rng, Rng};
use ratatui::{
    layout::Rect,
    terminal::Frame,
    widgets::{Block, List},
};

use crate::{
    constants::SYMBOLS,
    schema::lobby::{Encryption, Lobby},
};

pub fn draw_lobby(f: &mut Frame, area: Rect, lobby: &Lobby) {
    let block = Block::bordered().title("Lobby");
    let encrypted_names = lobby.encryptions.values().map(
        |Encryption {
             action: _,
             index,
             name,
         }| {
            if name.eq(&lobby.username) {
                return format!("{} (You)", name);
            }
            name.chars()
                .enumerate()
                .map(|(i, c)| if i < *index { c } else { get_random_symbol() })
                .collect::<String>()
        },
    );
    let players = List::new(encrypted_names).block(block);
    f.render_widget(players, area);
}

fn get_random_symbol() -> char {
    let mut rng = thread_rng();
    let idx = rng.gen_range(0..SYMBOLS.len());
    SYMBOLS.chars().nth(idx).unwrap()
}
