use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{block::Title, Block},
    Frame,
};
use tui_term::widget::PseudoTerminal;

use crate::{app::App, schema::focused_component::FocusedComponent};

pub fn draw_editor(f: &mut Frame, app: &App, area: Rect) {
    let focus_editor_key = format!("<{}>", app.config.key_bindings.lobby.focus_editor.code);
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from(focus_editor_key).alignment(Alignment::Right));

    if let Some(FocusedComponent::Editor) = app.focused_component {
        block = block.border_style(Style::default().fg(Color::Green));
    } else {
        block = block.border_style(Style::default().fg(Color::White));
    }
    if let Some(ref editor) = app.editor {
        let parser = editor.parser.lock().expect("Unable to lock editor parser");
        let terminal = PseudoTerminal::new(parser.screen()).block(block);
        f.render_widget(terminal, area);
    } else {
        f.render_widget(block, area);
    }
}
