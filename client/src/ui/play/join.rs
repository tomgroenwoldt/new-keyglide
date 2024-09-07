use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style, Stylize},
    widgets::{block::Title, Block, Cell, Row, Table},
    Frame,
};

use crate::{
    app::App,
    schema::{focused_component::ComponentKind, join::Join},
    ui::get_random_symbol,
};

pub fn draw_join(f: &mut Frame, app: &App, area: Rect, join: &Join) {
    let focus_lobby_key = format!("{}", app.config.key_bindings.join.focus_lobby_list);
    let mut block = Block::bordered()
        .title("Lobbies")
        .title(Title::from(focus_lobby_key).alignment(Alignment::Right));

    if app.focused_component_is_kind(ComponentKind::Lobbies) {
        block = block.border_style(Style::default().fg(Color::Green));
    }

    let rows = join
        .encrypted_names
        .iter()
        .zip(join.encrypted_player_counts.values())
        .zip(join.encrypted_status.values())
        .map(|(((id, name), player_count), status)| {
            let encrypted_name = name
                .value
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i < name.index {
                        c
                    } else {
                        get_random_symbol()
                    }
                })
                .collect::<String>();
            let encrypted_player_count = player_count
                .value
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i < player_count.index {
                        c
                    } else {
                        get_random_symbol()
                    }
                })
                .collect::<String>();
            let encrypted_status = status
                .value
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i < status.index {
                        c
                    } else {
                        get_random_symbol()
                    }
                })
                .collect::<String>();
            let mut row = Row::new(vec![
                Cell::from(encrypted_name),
                Cell::from(encrypted_player_count),
                Cell::from(encrypted_status),
            ]);
            if join.selected_lobby.is_some_and(|lobby_id| lobby_id.eq(id)) {
                row = row.style(Style::default().fg(Color::Yellow));
            }
            row
        });
    // Columns widths are constrained in the same way as Layout...
    let widths = [
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(
            Row::new(vec!["Name", "Player count", "Status"])
                .style(Style::new().bold())
                .bottom_margin(1),
        )
        .block(block);

    f.render_widget(table, area);
}
