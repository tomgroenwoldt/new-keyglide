use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{self},
    widgets::{Block, Tabs},
    Frame,
};
use strum::IntoEnumIterator;

use crate::{app::App, constants::APP_TITLE, schema::tab::Tab};

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    // Display all tabs in the header. Highlight the current selection.
    let tabs = Tab::iter()
        .map(|t| text::Line::from(t.to_string()))
        .collect::<Tabs>()
        .block(Block::bordered().title(APP_TITLE))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.current_tab.index());
    f.render_widget(tabs, area);
}
