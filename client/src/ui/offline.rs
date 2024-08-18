use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Wrap},
    Frame,
};

use super::centered_rect;
use crate::{constants::RECONNECT_INTERVAL, schema::offline::Offline};

pub fn draw_offline(f: &mut Frame, offline: &Offline) {
    let popup = Block::bordered()
        .title("Service offline")
        .border_style(Style::default().fg(Color::LightYellow));
    let text = "It appears we are offline. You can keep this window open. We will try to reconnect automatically.";

    // Calculate the amount of seconds that remain to start the reconnect.
    let since_last_reconnected = offline.last_reconnect.elapsed();
    let reconnecting_in = RECONNECT_INTERVAL - since_last_reconnected;
    let reconnect_status = &format!(
        "Trying to reconnect in {}s{}",
        reconnecting_in.as_secs(),
        ".".repeat(offline.dot_count)
    );

    let lines = [text, "", reconnect_status]
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();

    let area = centered_rect(f.area(), 30, 6);
    let paragraph = Paragraph::new(lines).block(popup).wrap(Wrap { trim: true });

    // Clear the area for the offline UI.
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
