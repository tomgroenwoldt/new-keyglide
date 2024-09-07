use ratatui::{
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::config::Config;

use super::centered_rect;

pub fn draw_exit(f: &mut Frame, config: &Config) {
    let popup = Block::bordered()
        .title("Exit?")
        .border_style(Style::default().fg(Color::Black));
    let text = format!(
        "Confirm {}, Abort {}",
        config.key_bindings.popup.confirm, config.key_bindings.popup.abort
    );
    let area = centered_rect(f.area(), text.len() as u16, 1);
    let paragraph = Paragraph::new(text)
        .block(popup)
        .style(Style::default().bg(Color::LightRed).fg(Color::Black));
    f.render_widget(paragraph, area);
}
