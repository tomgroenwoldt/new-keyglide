use ratatui::{
    style::{Color, Style},
    terminal::Frame,
    widgets::{Block, Paragraph},
};

use super::centered_rect;

pub fn draw_exit(f: &mut Frame) {
    let popup = Block::bordered()
        .title("Exit?")
        .border_style(Style::default().fg(Color::LightRed));
    let text = "Yes <y>, No <n>";
    let paragraph = Paragraph::new(text).block(popup);
    let area = centered_rect(f.size(), text.len() as u16, 1);
    f.render_widget(paragraph, area);
}
