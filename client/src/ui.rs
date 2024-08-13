use rand::{thread_rng, Rng};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    terminal::Frame,
    widgets::{Block, Paragraph, Wrap},
};

use crate::{
    app::{App, Connection},
    constants::SYMBOLS,
    schema::{focused_component::FocusedComponent, tab::Tab},
};

use self::{exit::draw_exit, header::draw_header, play::draw_play_tab};

mod exit;
mod header;
mod play;

pub fn draw(f: &mut Frame, app: &mut App) {
    if let Connection::Offline = app.connection {
        draw_offline(f);
    } else {
        draw_application(f, app);
    }

    // Optionally, render an exit popup above the current content.
    if let Some(FocusedComponent::ExitPopup) = app.focused_component {
        draw_exit(f);
    }
}

/// # Draw the application
///
/// Draws the application. Divides the layout into a header and content field.
pub fn draw_application(f: &mut Frame, app: &mut App) {
    // Split the layout into header and content.
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(f.size());

    draw_header(f, app, chunks[0]);

    // Render content depending on the selected tab.
    match app.current_tab {
        Tab::Home => {}
        Tab::Play => draw_play_tab(f, app, chunks[1]),
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

fn draw_offline(f: &mut Frame) {
    let popup = Block::bordered()
        .title("Service offline")
        .border_style(Style::default().fg(Color::LightYellow));
    let text = "It appears we are offline. You can keep this window open. We will try to reconnect automatically.";

    let area = centered_rect(f.size(), 30, 4);
    let paragraph = Paragraph::new(text).block(popup).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}
