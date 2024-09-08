use ratatui::{
    layout::{Alignment, Constraint, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{block::Title, Block, Cell, Row, Scrollbar, ScrollbarOrientation, Table},
    Frame,
};

use crate::{
    config::Config,
    schema::{
        focused_component::{ComponentKind, FocusedComponent},
        join::Join,
    },
    ui::get_random_symbol,
};

pub fn draw_join(
    f: &mut Frame,
    config: &Config,
    area: Rect,
    join: &mut Join,
    focused_component: &Option<FocusedComponent>,
) {
    let focus_lobby_key = format!("{}", config.key_bindings.join.focus_lobby_list);
    let mut block = Block::bordered()
        .title("Lobbies")
        .title(Title::from(focus_lobby_key).alignment(Alignment::Right));

    if focused_component
        .as_ref()
        .is_some_and(|component| component.kind.eq(&ComponentKind::Lobbies))
    {
        block = block.border_style(Style::default().fg(Color::Green));
    }

    let rows = join
        .encrypted_names
        .iter()
        .zip(join.encrypted_player_counts.values())
        .zip(join.encrypted_status.values())
        .map(|(((_, name), player_count), status)| {
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
            let row = Row::new(vec![
                Cell::from(encrypted_name),
                Cell::from(encrypted_player_count),
                Cell::from(encrypted_status),
            ]);
            row
        });
    // Columns widths are constrained in the same way as Layout...
    let widths = [
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ];
    let selected_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(Color::DarkGray);
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(
            Row::new(vec!["Name", "Player count", "Status"])
                .style(Style::new().bold())
                .bottom_margin(1),
        )
        .block(block)
        .highlight_style(selected_style);

    f.render_stateful_widget(table, area, &mut join.state);
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut join.scroll_state,
    );
}
