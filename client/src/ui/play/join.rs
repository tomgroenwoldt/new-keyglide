use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{block::Title, Block, List},
    Frame,
};

use crate::{
    app::App,
    schema::{encryption::Encryption, focused_component::FocusedComponent, join::Join},
    ui::get_random_symbol,
};

pub fn draw_join(f: &mut Frame, app: &App, area: Rect, join: &Join) {
    let focus_lobby_key = format!("<{}>", app.config.key_bindings.join.focus_lobby_list.code);
    let mut block = Block::bordered()
        .title("Lobbies")
        .title(Title::from(focus_lobby_key).alignment(Alignment::Right));

    if let Some(FocusedComponent::Lobbies) = app.focused_component {
        block = block.border_style(Style::default().fg(Color::Green));
    } else {
        block = block.border_style(Style::default().fg(Color::White));
    }

    let encrypted_names = join.encryptions.iter().map(
        |(
            id,
            Encryption {
                action: _,
                index,
                value,
            },
        )| {
            let encrypted_name = value
                .chars()
                .enumerate()
                .map(|(i, c)| if i < *index { c } else { get_random_symbol() })
                .collect::<String>();
            let mut line = Line::from(encrypted_name);
            if join.selected_lobby.is_some_and(|lobby_id| lobby_id.eq(id)) {
                line = line.style(Style::default().fg(Color::Yellow));
            }
            line
        },
    );

    let lobbies = List::new(encrypted_names).block(block);
    f.render_widget(lobbies, area);
}
