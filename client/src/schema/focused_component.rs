use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::{connection::Connection, join::Join};
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
                if let Connection::Join(_) = app.connection {
                    // Unfortunately, we have to pass the whole app in this
                    // function because the join component changes state
                    // outside of its scope.
                    Join::handle_key_event(app, key).await?;
                }
            }
            FocusedComponent::ExitPopup => {
                if let KeyCode::Char(c) = key.code {
                    match c {
                        'y' => {
                            app.exit = true;
                        }
                        'n' => {
                            app.focused_component = None;
                        }
                        _ => {}
                    }
                }
            }
        };
        Ok(())
    }
}
