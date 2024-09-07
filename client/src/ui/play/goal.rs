use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{block::Title, Block},
    Frame,
};
use tui_term::widget::PseudoTerminal;

use crate::{
    app::App,
    schema::{connection::Connection, focused_component::ComponentKind},
};

pub fn draw_goal(f: &mut Frame, app: &App, area: Rect) {
    let focus_goal_key = format!("{}", app.config.key_bindings.lobby.focus_goal);
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from(focus_goal_key).alignment(Alignment::Right));

    if app.focused_component_is_kind(ComponentKind::Goal) {
        block = block.border_style(Style::default().fg(Color::Green));
    }
    let Connection::Lobby(ref lobby) = app.connection else {
        return;
    };
    let parser = lobby
        .goal
        .terminal
        .parser
        .lock()
        .expect("Unable to lock editor parser");
    let terminal = PseudoTerminal::new(parser.screen()).block(block);
    f.render_widget(terminal, area);
}
