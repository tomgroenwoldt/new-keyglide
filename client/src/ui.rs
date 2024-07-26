use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    terminal::Frame,
};

use crate::{
    app::{App, FocusedComponent},
    tab::Tab,
};

use self::{exit::draw_exit, header::draw_header, play::draw_play_tab};

mod exit;
mod header;
mod play;

pub fn draw(f: &mut Frame, app: &App) {
    // Split the layout into header and content.
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(f.size());

    draw_header(f, app, chunks[0]);

    // Render content depending on the selected tab.
    match app.current_tab {
        Tab::Home => {}
        Tab::Play => draw_play_tab(f, app, chunks[1]),
    };

    // Optionally, render an exit popup above the current content.
    if let Some(FocusedComponent::ExitPopup) = app.focused_component {
        draw_exit(f);
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
    let horizontal_length = content_length + 4;

    let popup_layout = Layout::vertical([Constraint::Length(vertical_length)])
        .flex(Flex::Center)
        .split(r);

    Layout::horizontal([Constraint::Length(horizontal_length)])
        .flex(Flex::Center)
        .split(popup_layout[0])[0]
}
