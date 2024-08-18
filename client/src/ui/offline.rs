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
    let reconnect_status =
        if let Some(reconnecting_in) = RECONNECT_INTERVAL.checked_sub(since_last_reconnected) {
            let millis = reconnecting_in.as_millis();
            let seconds_with_millis = millis as f64 / 1000.0;
            &format!(
                "Trying to reconnect in {:.1}s{}",
                seconds_with_millis,
                ".".repeat(offline.dot_count)
            )
        } else {
            &format!("Trying to reconnect{}", ".".repeat(offline.dot_count))
        };
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
