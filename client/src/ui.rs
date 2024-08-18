use logs::draw_logs_tab;
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
    schema::{connection::Connection, focused_component::FocusedComponent, tab::Tab},
};

mod exit;
mod header;
mod home;
mod logs;
mod offline;
mod play;

pub fn draw(f: &mut Frame, app: &mut App) {
    draw_application(f, app);

    // Optionally, render an exit popup above the current content.
    if let Some(FocusedComponent::ExitPopup) = app.focused_component {
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

    // If we are offline just draw the offline UI above everything else.
    if let Connection::Offline(ref offline) = app.connection {
        draw_offline(f, offline);
    }

    // Render content depending on the selected tab.
    match app.current_tab {
        Tab::Home => draw_home_tab(f, app, chunks[1]),
        Tab::Play => draw_play_tab(f, app, chunks[1]),
        Tab::Logs => draw_logs_tab(f, chunks[1]),
    };
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

pub fn get_random_symbol() -> char {
    let mut rng = thread_rng();
    let idx = rng.gen_range(0..SYMBOLS.len());
    SYMBOLS.chars().nth(idx).unwrap()
}
