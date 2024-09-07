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

pub fn draw_editor(f: &mut Frame, app: &App, area: Rect) {
    let focus_editor_key = format!("{}", app.config.key_bindings.lobby.focus_editor);
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from(focus_editor_key).alignment(Alignment::Right));

    if app.focused_component_is_kind(ComponentKind::Editor) {
        block = block.border_style(Style::default().fg(Color::Green));
    }
    let Connection::Lobby(ref lobby) = app.connection else {
        return;
    };
    let parser = lobby
        .editor
        .terminal
        .parser
        .lock()
        .expect("Unable to lock editor parser");
    let terminal = PseudoTerminal::new(parser.screen()).block(block);
    f.render_widget(terminal, area);
}
