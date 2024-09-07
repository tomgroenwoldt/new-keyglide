use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{block::Title, Block, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::App,
    schema::{focused_component::ComponentKind, lobby::Lobby},
};

pub fn draw_chat(f: &mut Frame, app: &App, area: Rect, lobby: &Lobby) {
    let focus_chat_key = format!("{}", app.config.key_bindings.lobby.focus_chat);
    let mut block = Block::bordered()
        .title("Chat")
        .title(Title::from(focus_chat_key).alignment(Alignment::Right));
    let mut input_text = format!("You: {}", lobby.chat.input.clone());
    let inner_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(area);
    if app.focused_component_is_kind(ComponentKind::Chat) {
        block = block.border_style(Style::default().fg(Color::Green));
        input_text.push('|');
    }
    let input = Paragraph::new(input_text).wrap(Wrap::default());
    let messages = lobby.chat.to_lines();
    let paragraph = Paragraph::new(messages).block(block).wrap(Wrap::default());
    f.render_widget(paragraph, area);
    f.render_widget(
        input,
        inner_chunks[1].inner(Margin {
            horizontal: 1,
            vertical: 0,
        }),
    )
}
