use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Block,
    Frame,
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

pub fn draw_logs_tab(f: &mut Frame, area: Rect) {
    let block = Block::bordered().title("Logger");
    let logger = TuiLoggerWidget::default()
        .block(block)
        .style_error(Style::default().fg(Color::LightRed))
        .style_debug(Style::default().fg(Color::DarkGray))
        .style_warn(Style::default().fg(Color::LightYellow))
        .style_info(Style::default().fg(Color::LightGreen))
        .output_separator(' ')
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        // Do not display the file and the line number.
        .output_file(false)
        .output_line(false);

    f.render_widget(logger, area);
}
