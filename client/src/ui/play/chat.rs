use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{block::Title, Block, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{
    config::Config,
    schema::{
        chat::Chat,
        focused_component::{ComponentKind, FocusedComponent},
    },
};

pub fn draw_chat(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    chat: &mut Chat,
    focused_component: &Option<FocusedComponent>,
) {
    let move_down_key = format!("{}", config.key_bindings.movement.down);
    let move_up_key = format!("{}", config.key_bindings.movement.up);
    let block = Block::bordered()
        .title("Chat")
        .title(Title::from(move_down_key).alignment(Alignment::Right))
        .title(Title::from(move_up_key).alignment(Alignment::Right));

    // If the chat is focused change the block border color to green.
    let focus_chat_key = format!("{}", config.key_bindings.lobby.focus_chat);
    let mut input_block = Block::bordered()
        .title("Message")
        .title(Title::from(focus_chat_key).alignment(Alignment::Right));
    let mut input_text = chat.input.to_string();
    if focused_component
        .as_ref()
        .is_some_and(|component| component.kind.eq(&ComponentKind::Chat))
    {
        input_block = input_block.border_style(Style::default().fg(Color::Green));
        input_text.push('|');
    }

    let input = Paragraph::new(input_text)
        .block(input_block)
        .wrap(Wrap::default());

    // Setup layout.
    let input_height = input.line_count((area.columns().count() - 2) as u16);
    let chunks =
        Layout::vertical([Constraint::Min(0), Constraint::Length(input_height as u16)]).split(area);

    // Render input widget.
    f.render_widget(input, chunks[1]);

    let chat_width = chunks[0].width - 2;
    let messages: Vec<Row> = chat
        .messages
        .iter()
        .map(|msg| {
            let (formatted_text, height) = insert_newlines(msg, chat_width as usize);
            Row::new([Cell::from(Text::from(formatted_text))]).height(height)
        })
        .collect();
    let selected_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(Color::DarkGray);
    let table = Table::new(messages, [Constraint::Min(0)])
        .block(block)
        .highlight_style(selected_style);

    // Render messages widget.
    f.render_stateful_widget(table, chunks[0], &mut chat.state);
}

fn insert_newlines(text: &str, width: usize) -> (String, u16) {
    let words = text.split_whitespace(); // Split the text into words
    let mut result = String::new(); // Store the result
    let mut line = String::new(); // Current line
    let mut height = 1;

    for word in words {
        if word.len() > width {
            // If the word itself is longer than the width, break it into chunks
            if !line.is_empty() {
                // Add the current line to the result before breaking the word
                result.push_str(line.trim_end());
                result.push('\n');
                height += 1;
                line = String::new();
            }

            // Break the long word into chunks and add each to the result
            let word_chars: Vec<_> = word.chars().collect();
            let chunk_count = (word.len() + width - 1) / width; // Number of chunks
            for (i, chunk) in word_chars.chunks(width).enumerate() {
                result.push_str(&chunk.iter().collect::<String>());
                if i < chunk_count - 1 {
                    result.push('\n'); // Insert newline after every chunk except the last one
                    height += 1;
                }
            }
        } else {
            // If adding the next word exceeds the width
            if line.len() + word.len() + 1 > width {
                // Add the line to the result and start a new line
                result.push_str(line.trim_end());
                result.push('\n');
                height += 1;
                line = String::new();
            }

            // Append the word to the current line
            if !line.is_empty() {
                line.push(' '); // Add space before the word if not the first word
            }
            line.push_str(word);
        }
    }

    // Add the last line to the result
    if !line.is_empty() {
        result.push_str(line.trim_end());
    }

    (result, height)
}
