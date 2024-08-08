use std::collections::BTreeMap;

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::error;
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage, Player};

use crate::lobby::Lobby;

#[derive(Debug)]
pub struct AppState {
    /// All connected clients that are not in a lobby.
    pub clients: BTreeMap<Uuid, UnboundedSender<BackendMessage>>,
    /// All active lobbies.
    pub lobbies: BTreeMap<Uuid, Lobby>,
    /// The sending part of the application's main channel. This is used to send
    /// messages to different parts of the application.
    pub tx: UnboundedSender<AppMessage>,
    pub rx: UnboundedReceiver<AppMessage>,
}

impl AppState {
    /// # Create a new app state
    ///
    /// Createa a new app state. `tx`
    pub fn new(tx: UnboundedSender<AppMessage>, rx: UnboundedReceiver<AppMessage>) -> Self {
        Self {
            clients: BTreeMap::default(),
            lobbies: BTreeMap::default(),
            tx,
            rx,
        }
    }

    /// # Add player via quickplay
    ///
    /// Adds the player to the first nonfull lobby. Creates a new lobby if no
    /// nonfull lobby was found.
    pub fn add_player_via_quickplay(&mut self, player_id: Uuid, player: Player) -> Result<()> {
        if let Some(lobby) = self
            .lobbies
            .values_mut()
            .find(|lobby| lobby.players.len() < MAX_LOBBY_SIZE)
        {
            lobby.add_player(player_id, player, &self.tx)?;
        } else {
            let mut lobby = Lobby::default();
            lobby.add_player(player_id, player, &self.tx)?;
            self.lobbies.insert(lobby.id, lobby);
        }
        Ok(())
    }

    /// # Get current lobbies
    ///
    /// Fetches all active lobbies and returns them inside a `BTreeMap`. Lobbies
    /// are converted into the client compatible `common::Lobby` type.
    pub fn get_current_lobbies(&self) -> BTreeMap<Uuid, common::Lobby> {
        let mut lobbies = BTreeMap::new();
        for lobby in self.lobbies.values() {
            lobbies.insert(lobby.id, lobby.to_common_lobby());
        }
        lobbies
    }

    /// # Remove lobby
    ///
    /// Removes a lobby if it exists. All connected clients are informed about
    /// the removed lobby.
    pub fn remove_lobby(&mut self, lobby_id: Uuid) -> Result<()> {
        if self.lobbies.remove(&lobby_id).is_some() {
            for client in self.clients.values() {
                client.send(BackendMessage::RemoveLobby(lobby_id))?;
            }
        }
        Ok(())
    }

    /// # Send lobby information
    ///
    /// Sends the lobby information to every connected client. This is used to
    /// keep clients up to date to available lobbies.
    pub fn send_lobby_information(&self, lobby_id: Uuid) -> Result<()> {
        if let Some(lobby) = self.lobbies.get(&lobby_id) {
            for client in self.clients.values() {
                client.send(BackendMessage::AddLobby(lobby_id, lobby.to_common_lobby()))?;
            }
        }
        Ok(())
    }
}

pub enum AppMessage {
    // Lobby messages.
    CurrentPlayers {
        lobby_id: Uuid,
        player_id: Uuid,
    },
    AddPlayerViaQuickplay {
        player_id: Uuid,
        player: Player,
    },
    AddPlayerToLobby {
        lobby_id: Uuid,
        player_id: Uuid,
        player: Player,
    },
    SendMessage {
        player_id: Uuid,
        message: String,
    },
    RemovePlayer {
        player_id: Uuid,
    },
    LobbyFull {
        player_tx: UnboundedSender<BackendMessage>,
    },

    // Client messages.
    CurrentLobbies {
        client_id: Uuid,
    },
    SendLobbyInformation {
        lobby_id: Uuid,
    },
    RemoveLobby {
        lobby_id: Uuid,
    },

    AddClient {
        client_id: Uuid,
        client_tx: UnboundedSender<BackendMessage>,
    },
    RemoveClient {
        client_id: Uuid,
    },
}

/// # Handle app message
///
/// Manages the app state based on received `AppMessage`.
pub async fn handle_app_message(mut app_state: AppState) -> Result<()> {
    while let Some(msg) = app_state.rx.recv().await {
        match msg {
            AppMessage::CurrentPlayers {
                lobby_id,
                player_id,
            } => {
                if let Some(lobby) = app_state.lobbies.get(&lobby_id) {
                    lobby.send_current_players(player_id)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::AddPlayerViaQuickplay { player_id, player } => {
                app_state.add_player_via_quickplay(player_id, player)?;
            }
            AppMessage::AddPlayerToLobby {
                lobby_id,
                player_id,
                player,
            } => {
                if let Some(lobby) = app_state.lobbies.get_mut(&lobby_id) {
                    lobby.add_player(player_id, player, &app_state.tx)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::SendMessage { player_id, message } => {
                for lobby in app_state.lobbies.values_mut() {
                    lobby.send_message(player_id, message.clone())?;
                }
            }
            AppMessage::RemovePlayer { player_id } => {
                for lobby in app_state.lobbies.values_mut() {
                    lobby.remove_player(player_id, &app_state.tx)?;
                }
            }
            AppMessage::LobbyFull { player_tx } => {
                let message = BackendMessage::LobbyFull;
                player_tx.send(message)?;
            }

            AppMessage::CurrentLobbies { client_id } => {
                if let Some(client) = app_state.clients.get(&client_id) {
                    let lobbies = app_state.get_current_lobbies();
                    let message = BackendMessage::CurrentLobbies(lobbies);
                    client.send(message)?;
                } else {
                    error!("Client with ID {} was not found.", client_id);
                }
            }
            AppMessage::SendLobbyInformation { lobby_id } => {
                app_state.send_lobby_information(lobby_id)?;
            }
            AppMessage::RemoveLobby { lobby_id } => {
                app_state.remove_lobby(lobby_id)?;
            }

            AppMessage::AddClient {
                client_id,
                client_tx,
            } => {
                app_state.clients.insert(client_id, client_tx);
            }
            AppMessage::RemoveClient { client_id } => {
                app_state.clients.remove(&client_id);
            }
        }
    }
    Ok(())
}
