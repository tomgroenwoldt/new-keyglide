use anyhow::{anyhow, Result};
use log::debug;
use ratatui::crossterm::event::KeyEvent;

use super::connection::Connection;
use crate::app::App;

pub struct FocusedComponent {
    pub kind: ComponentKind,
    pub is_full_screen: bool,
}

#[derive(PartialEq, Eq)]
pub enum ComponentKind {
    Chat,
    Editor,
    ExitPopup,
    Goal,
    Lobbies,
}

impl FocusedComponent {
    pub fn new(kind: ComponentKind) -> Self {
        Self {
            kind,
            is_full_screen: false,
        }
    }

    /// # Toggle full screen
    ///
    /// Toggles the full screen view of the focused component. Also handles the
    /// implied resize events for specific components, e.g., the editor.
    pub fn toggle_full_screen(app: &mut App) -> Result<()> {
        let Some(ref mut focused_component) = app.focused_component else {
            return Err(anyhow!(
                "Error trying to toggle full screen without a focused component."
            ));
        };

        focused_component.is_full_screen = !focused_component.is_full_screen;

        match focused_component.kind {
            ComponentKind::Chat => {}
            ComponentKind::Editor => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.editor.is_full_screen = focused_component.is_full_screen;
                    lobby.editor.resize(app.size.height, app.size.width)?;
                }
            }
            ComponentKind::Goal => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.goal.is_full_screen = focused_component.is_full_screen;
                    lobby.goal.resize(app.size.height, app.size.width)?;
                }
            }
            ComponentKind::Lobbies => {}
            ComponentKind::ExitPopup => {}
        };
        Ok(())
    }

    pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
        debug!("Handle key event {:?}.", key);
        let Some(ref mut focused_component) = app.focused_component else {
            return Err(anyhow!(
                "Error trying handle key event of non-existent focused component."
            ));
        };

        // Return early when the user toggles full screen to avoid triggering
        // other key event handlers.
        if key.eq(&app.config.key_bindings.miscellaneous.toggle_full_screen) {
            FocusedComponent::toggle_full_screen(app)?;
            return Ok(());
        }

        match focused_component.kind {
            ComponentKind::Chat => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.chat.handle_key_event(key)?;
                }
            }
            ComponentKind::Editor => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.editor.terminal.handle_key_event(key)?;
                }
            }
            ComponentKind::Goal => {}
            ComponentKind::Lobbies => {
                if let Connection::Join(ref mut join) = app.connection {
                    join.handle_key_event(&app.config, key).await?;
                }
            }
            ComponentKind::ExitPopup => {
                if key.eq(&app.config.key_bindings.popup.confirm) {
                    app.exit = true;
                } else if key.eq(&app.config.key_bindings.popup.abort) {
                    app.focused_component = None;
                }
            }
        };
        Ok(())
    }

    /// # Clean up
    ///
    /// Cleans up the focused component that is about to be dropped. For now,
    /// this is only the editor that needs to be resized when unfocused.
    pub fn clean_up(app: &mut App) -> Result<()> {
        let Some(ref mut focused_component) = app.focused_component else {
            return Err(anyhow!(
                "Error trying clean up non-existent focused component."
            ));
        };
        match focused_component.kind {
            ComponentKind::Chat => {}
            // In case of a focused editor, tell the actual editor instance it's
            // not full screen anymore and resize it.
            ComponentKind::Editor => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.editor.is_full_screen = false;
                    lobby.editor.resize(app.size.height, app.size.width)?;
                }
            }
            // In case of a focused editor, tell the actual editor instance it's
            // not full screen anymore and resize it.
            ComponentKind::Goal => {
                if let Connection::Lobby(ref mut lobby) = app.connection {
                    lobby.goal.is_full_screen = false;
                    lobby.goal.resize(app.size.height, app.size.width)?;
                }
            }
            ComponentKind::Lobbies => {}
            ComponentKind::ExitPopup => {}
        };
        Ok(())
    }
}
