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
        focused_component::{ComponentKind, FocusedComponent},
        goal::Goal,
    },
};

pub fn draw_goal(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    goal: &Goal,
    focused_component: &Option<FocusedComponent>,
) {
    let focus_goal_key = format!("{}", config.key_bindings.lobby.focus_goal);
    let mut block = Block::bordered()
        .title("Editor")
        .title(Title::from(focus_goal_key).alignment(Alignment::Right));

    if focused_component
        .as_ref()
        .is_some_and(|component| component.kind.eq(&ComponentKind::Goal))
    {
        block = block.border_style(Style::default().fg(Color::Green));
    }
    let parser = goal
        .terminal
        .parser
        .lock()
        .expect("Unable to lock editor parser");
    let terminal = PseudoTerminal::new(parser.screen()).block(block);
    f.render_widget(terminal, area);
}
