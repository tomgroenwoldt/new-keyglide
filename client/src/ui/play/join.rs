use ratatui::{
    layout::{Alignment, Rect},
    terminal::Frame,
    widgets::{block::Title, Block, Paragraph},
};

use crate::ui::centered_rect;

pub fn draw_join(f: &mut Frame, area: Rect) {
    let text = "Press <j> to join the lobby.";
    let area = centered_rect(area, text.len() as u16, 1);
    let join = Block::bordered()
        .title("Join")
        .title(Title::from("<j>").alignment(Alignment::Right));
    let username = Paragraph::new(text).block(join).centered();
    f.render_widget(username, area);
}
