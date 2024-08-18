use std::collections::VecDeque;

use anyhow::Result;
use log::debug;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    text::Line,
};
use tokio::sync::mpsc::UnboundedSender;

use super::lobby::LobbyMessage;
use crate::constants::CHAT_SIZE;

pub struct Chat {
    messages: VecDeque<String>,
    pub input: String,
    pub message_tx: UnboundedSender<LobbyMessage>,
}

impl Chat {
    pub fn new(message_tx: UnboundedSender<LobbyMessage>) -> Self {
        Self {
            messages: VecDeque::new(),
            input: String::new(),
            message_tx,
        }
    }

    pub fn add_message(&mut self, message: String) {
        debug!("Add message {message} to chat.");
        self.messages.push_back(message);

        if self.messages.len() > CHAT_SIZE - 3 {
            self.messages.pop_front();
        }
    }

    pub fn to_lines(&self) -> Vec<Line> {
        let messages: Vec<Line> = self
            .messages
            .iter()
            .map(|msg| Line::from(msg.clone()))
            .collect();
        messages
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        debug!("Handle key event {:?}.", key);

        match key.code {
            KeyCode::Char(input) => {
                self.input.push(input);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                self.message_tx.send(LobbyMessage::SendMessage {
                    message: self.input.clone(),
                })?;
                self.input = String::new();
            }
            _ => {}
        };
        Ok(())
    }
}
