use logs::draw_logs_tab;
use play::{chat::draw_chat, editor::draw_editor, goal::draw_goal, join::draw_join};
use rand::{thread_rng, Rng};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    Frame,
};

use self::{
    exit::draw_exit, header::draw_header, home::draw_home_tab, offline::draw_offline,
    play::draw_play_tab,
};
use crate::{
    app::App,
    constants::SYMBOLS,
    schema::{connection::Connection, focused_component::ComponentKind, tab::Tab},
};

mod exit;
mod header;
mod home;
mod logs;
mod offline;
mod play;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Check if one component is set to full screen. If that's the case draw the
    // full screen component and return directly.
    if app
        .focused_component
        .as_ref()
        .is_some_and(|component| component.is_full_screen)
    {
        draw_full_screen(f, app);
        return;
    }

    draw_application(f, app);

    // Optionally, render an exit popup above the current content.
    if app.focused_component_is_kind(ComponentKind::ExitPopup) {
        draw_exit(f, &app.config);
    }
}

/// # Draw the application
///
/// Draws the application. Divides the layout into a header and content field.
pub fn draw_application(f: &mut Frame, app: &mut App) {
    // Split the layout into header and content.
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(f.area());

    draw_header(f, app, chunks[0]);

    // Render content depending on the selected tab.
    match app.current_tab {
        Tab::Home => draw_home_tab(f, app, chunks[1]),
        Tab::Play => draw_play_tab(f, app, chunks[1]),
        Tab::Logs => draw_logs_tab(f, chunks[1]),
    };

    // If we are offline just draw the offline UI above everything else.
    if let Connection::Offline(ref offline) = app.connection {
        draw_offline(f, offline);
    }
}

/// # Create a centered rectangle inside a given rectangle
///
/// Returns a rectangle centered inside the input rectangle. `content_length`
/// should be the number of columns your content needs to display. Respectively,
/// `content_height` should be the number of rows.
pub fn centered_rect(r: Rect, content_length: u16, content_height: u16) -> Rect {
    // Add padding for potential borders of blocks.
    let vertical_length = content_height + 2;
    let horizontal_length = content_length + 2;

    let popup_layout = Layout::vertical([Constraint::Length(vertical_length)])
        .flex(Flex::Center)
        .split(r);

    Layout::horizontal([Constraint::Length(horizontal_length)])
        .flex(Flex::Center)
        .split(popup_layout[0])[0]
}

/// # Draw full screen
///
/// Draws a focused component on the full screen.
pub fn draw_full_screen(f: &mut Frame, app: &mut App) {
    let Some(ref focused_component) = app.focused_component else {
        return;
    };

    let area = Rect::new(0, 0, app.size.width, app.size.height);
    match app.connection {
        Connection::Join(ref mut join) => match focused_component.kind {
            ComponentKind::Chat
            | ComponentKind::Editor
            | ComponentKind::Goal
            | ComponentKind::ExitPopup => {}
            ComponentKind::Lobbies => draw_join(f, &app.config, area, join, &app.focused_component),
        },
        Connection::Lobby(ref mut lobby) => match focused_component.kind {
            ComponentKind::Chat => draw_chat(
                f,
                area,
                &app.config,
                &mut lobby.chat,
                &app.focused_component,
            ),
            ComponentKind::Editor => {
                draw_editor(f, area, &app.config, &lobby.editor, &app.focused_component)
            }
            ComponentKind::Goal => {
                draw_goal(f, area, &app.config, &lobby.goal, &app.focused_component)
            }
            ComponentKind::ExitPopup => draw_exit(f, &app.config),
            ComponentKind::Lobbies => {}
        },
        Connection::Offline(_) => {}
    }
}

pub fn get_random_symbol() -> char {
    let mut rng = thread_rng();
    let idx = rng.gen_range(0..SYMBOLS.len());
    SYMBOLS.chars().nth(idx).unwrap_or('.')
}
