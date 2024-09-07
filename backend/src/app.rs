use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{error, info};
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage, JoinMode, LobbyListItem};

use self::message::AppMessage;
use crate::lobby::Lobby;

pub(crate) mod message;

#[derive(Debug)]
pub struct App {
    /// All non-playing clients.
    pub clients: BTreeMap<Uuid, UnboundedSender<BackendMessage>>,
    /// All active lobbies.
    pub lobbies: BTreeMap<Uuid, Lobby>,

    pub tx: UnboundedSender<AppMessage>,
    pub rx: UnboundedReceiver<AppMessage>,
}

impl App {
    /// # Create a new app
    ///
    /// Creates a new app with no clients and lobbies. Holds the passed in
    /// communication channel.
    pub fn new(tx: UnboundedSender<AppMessage>, rx: UnboundedReceiver<AppMessage>) -> Self {
        Self {
            clients: BTreeMap::default(),
            lobbies: BTreeMap::default(),
            tx,
            rx,
        }
    }

    /// # Get lobby ID
    ///
    /// Returns the ID of an available lobby or creates a new one depending on
    /// the provided `JoinMode`.
    pub fn get_lobby_id(&mut self, join_mode: JoinMode) -> Result<Uuid> {
        match join_mode {
            // Find a non-full lobby. If there is none, create a new one.
            JoinMode::Quickplay => {
                if let Some(lobby) = self
                    .lobbies
                    .values_mut()
                    .filter(|lobby| lobby.players.len() < MAX_LOBBY_SIZE)
                    .max_by_key(|lobby| lobby.players.len())
                {
                    Ok(lobby.id)
                } else {
                    self.create_new_lobby()
                }
            }
            // Try to join the lobby with the provided ID.
            JoinMode::Join { lobby_id } => {
                let Some(lobby) = self.lobbies.get_mut(&lobby_id) else {
                    return Err(anyhow!("Lobby with ID {} was not found in app state. Could not get lobby information.", lobby_id));
                };
                Ok(lobby.id)
            }
            // Create a new lobby.
            JoinMode::Create => self.create_new_lobby(),
        }
    }

    /// # Create new lobby
    ///
    /// Creates a new lobby and inserts it into the application state.
    pub fn create_new_lobby(&mut self) -> Result<Uuid> {
        // Create the new lobby.
        let lobby = Lobby::default();
        self.lobbies.insert(lobby.id, lobby.clone());
        self.tx.send(AppMessage::AddLobby { lobby_id: lobby.id })?;

        info!(
            "Created new lobby {}. {} open lobby/lobbies.",
            lobby.name,
            self.lobbies.len()
        );

        Ok(lobby.id)
    }

    /// # Get current lobbies
    ///
    /// Fetches all active lobbies and returns them inside a `BTreeMap`. Lobbies
    /// are converted into the client compatible `common::Lobby` type.
    pub fn get_current_lobbies(&self) -> BTreeMap<Uuid, LobbyListItem> {
        let mut lobbies = BTreeMap::new();
        for lobby in self.lobbies.values() {
            lobbies.insert(lobby.id, lobby.to_list_item());
        }
        lobbies
    }

    /// # Remove lobby
    ///
    /// Removes a lobby if it exists and it is empty. All connected clients are
    /// informed about the removed lobby.
    pub fn remove_lobby(&mut self, lobby_id: Uuid) -> Result<()> {
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            let error_message = format!("Lobby with ID {} was not found.", lobby_id);
            error!("{}", error_message);
            return Err(anyhow!(error_message));
        };
        if lobby.players.is_empty() {
            if let Some(lobby) = self.lobbies.remove(&lobby_id) {
                info!(
                    "Removed lobby {} with player count {}. Lobby count is {}.",
                    lobby.name,
                    lobby.players.len(),
                    self.lobbies.len(),
                );
                for client in self.clients.values() {
                    client.send(BackendMessage::RemoveLobby(lobby_id))?;
                }
            }
        } else {
            error!(
                "Can not remove non-empty lobby {} with {} players.",
                lobby.name,
                lobby.players.len()
            );
        }
        Ok(())
    }

    /// # Send lobby list information
    ///
    /// Sends the lobby list information to every connected client. This is used
    /// to keep clients up to date to available lobbies.
    pub fn send_lobby_list_information(&self, lobby_id: Uuid) -> Result<()> {
        let Some(lobby) = self.lobbies.get(&lobby_id) else {
            let error_message = format!("Lobby with ID {} was not found.", lobby_id);
            error!("{}", error_message);
            return Err(anyhow!(error_message));
        };
        for client in self.clients.values() {
            client.send(BackendMessage::AddLobby(lobby_id, lobby.to_list_item()))?;
        }
        Ok(())
    }
}
