use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{block::Title, Block},
    Frame,
};
use tui_term::widget::PseudoTerminal;

use crate::{
    config::Config,
    schema::{
        editor::Editor,
        focused_component::{ComponentKind, FocusedComponent},
    },
};

pub fn draw_editor(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    editor: &Editor,
    focused_component: &Option<FocusedComponent>,
) {
    let focus_editor_key = format!("{}", config.key_bindings.lobby.focus_editor);
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from(focus_editor_key).alignment(Alignment::Right));

    if focused_component
        .as_ref()
        .is_some_and(|component| component.kind.eq(&ComponentKind::Editor))
    {
        block = block.border_style(Style::default().fg(Color::Green));
    }
    let parser = editor
        .terminal
        .parser
        .lock()
        .expect("Unable to lock editor parser");
    let terminal = PseudoTerminal::new(parser.screen()).block(block);
    f.render_widget(terminal, area);
}
