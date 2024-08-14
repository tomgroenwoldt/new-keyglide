use ratatui::{
    layout::Rect,
    terminal::Frame,
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::{app::App, schema::connection::Connection};

use super::centered_rect;

pub fn draw_home_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let popup = Block::bordered().title("Live status");
    if let Connection::Join(ref join) = app.connection {
        let text = vec![
            Line::from(format!("Clients connected: {}", join.total_clients)),
            Line::from(format!("Players connected: {}", join.total_players)),
        ];
        let area = centered_rect(area, 25, 2);

        let paragraph = Paragraph::new(text).block(popup);
        f.render_widget(paragraph, area);
    }
}
