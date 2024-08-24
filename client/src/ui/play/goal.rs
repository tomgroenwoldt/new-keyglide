use ratatui::{layout::Rect, widgets::Block, Frame};
use tui_term::widget::PseudoTerminal;

use crate::schema::lobby::Lobby;

pub fn draw_goal(f: &mut Frame, lobby: &Lobby, area: Rect) {
    let block = Block::bordered().title("Goal");

    let parser = lobby
        .goal
        .terminal
        .parser
        .lock()
        .expect("Unable to lock editor parser");
    let terminal = PseudoTerminal::new(parser.screen()).block(block);
    f.render_widget(terminal, area);
}
