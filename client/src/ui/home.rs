use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;

use super::centered_rect;

pub fn draw_home_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let popup = Block::bordered().title("Live status");
    let text = vec![
        Line::from(format!("Clients connected: {}", app.total_clients)),
        Line::from(format!("Players connected: {}", app.total_players)),
    ];
    let area = centered_rect(area, 25, 2);

    let paragraph = Paragraph::new(text).block(popup);
    f.render_widget(paragraph, area);
}
