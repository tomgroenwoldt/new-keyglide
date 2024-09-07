use anyhow::Result;
use log::debug;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    widgets::TableState,
};
use tokio::sync::mpsc::UnboundedSender;

use super::lobby::LobbyMessage;

pub struct Chat {
    pub messages: Vec<String>,
    pub input: String,
    pub message_tx: UnboundedSender<LobbyMessage>,
    pub state: TableState,
}

impl Chat {
    pub fn new(message_tx: UnboundedSender<LobbyMessage>) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            message_tx,
            state: TableState::default(),
        }
    }

    pub fn add_message(&mut self, message: String) {
        debug!("Add message '{message}' to chat.");
        self.messages.push(message);
        self.state.scroll_down_by(1);
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    return;
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    return;
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
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
