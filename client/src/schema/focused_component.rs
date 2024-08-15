use anyhow::Result;
use ratatui::crossterm::event::KeyEvent;

use super::connection::Connection;
use crate::app::App;

#[derive(Clone)]
pub enum FocusedComponent {
    Chat,
    Editor,
    ExitPopup,
    Lobbies,
}

impl FocusedComponent {
    pub async fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match self {
            FocusedComponent::Chat => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.chat.handle_key_event(key)?;
                }
            }
            FocusedComponent::Editor => {
                if let Some(ref mut editor) = app.editor {
                    editor.handle_key_event(key)?;
                }
            }
            FocusedComponent::Lobbies => {
                if let Connection::Join(ref mut join) = app.connection {
                    join.handle_key_event(&app.config, key).await?;
                }
            }
            FocusedComponent::ExitPopup => {
                if key.eq(&app.config.key_bindings.popup.confirm) {
                    app.exit = true;
                } else if key.eq(&app.config.key_bindings.popup.abort) {
                    app.focused_component = None;
                }
            }
        };
        Ok(())
    }
}
