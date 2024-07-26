use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    terminal::Frame,
    widgets::{block::Title, Block},
};
use tui_term::widget::PseudoTerminal;

use crate::app::{App, FocusedComponent};

pub fn draw_editor(f: &mut Frame, app: &App, area: Rect) {
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from("<s>").alignment(Alignment::Right));

    if let Some(FocusedComponent::Editor) = app.focused_component {
        block = block.border_style(Style::default().fg(Color::Green));
    } else {
        block = block.border_style(Style::default().fg(Color::White));
    }
    if let Some(ref editor) = app.editor {
        let parser = editor.parser.lock().unwrap();
        let terminal = PseudoTerminal::new(parser.screen()).block(block);
        f.render_widget(terminal, area);
    } else {
        f.render_widget(block, area);
    }
}
