use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    terminal::Frame,
    widgets::{block::Title, Block, Paragraph, Wrap},
};

use crate::{
    app::{App, FocusedComponent},
    schema::lobby::Lobby,
};

pub fn draw_chat(f: &mut Frame, app: &App, area: Rect, lobby: &Lobby) {
    let mut block = Block::bordered()
        .title("Chat")
        .title(Title::from("<i>").alignment(Alignment::Right));
    let mut input_text = format!("You: {}", lobby.chat.input.clone());
    let inner_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(area);
    if let Some(FocusedComponent::Chat) = app.focused_component {
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
