use anyhow::Result;
use tokio::sync::{mpsc::UnboundedSender, oneshot::Sender};
use tracing::{error, info};
use uuid::Uuid;

use common::{BackendMessage, JoinMode, LobbyInformation};

use super::App;
use crate::player::Player;

pub enum AppMessage {
    /// Provide lobby information to the client who wants to play. Depending on
    /// the join mode this leads to the inspection of an already running lobby
    /// or the creation of a new one.
    ProvideLobbyInformation {
        tx: Sender<LobbyInformation>,
        join_mode: JoinMode,
    },
    AddPlayerToLobby {
        lobby_id: Uuid,
        player: Player,
    },
    /// Removes a player from the lobby and broadcasts this information to
    /// already connected players.
    RemovePlayer {
        player: Player,
    },
    /// Broadcasts a message of provided player to all connected players.
    SendMessage {
        player: Player,
        message: String,
    },

    /// Broadcasts all existing lobbies to a freshly connected client.
    CurrentLobbies {
        client_id: Uuid,
    },
    /// Broadcasts name and player count of a lobby to all connected clients.
    SendLobbyListInformation {
        lobby_id: Uuid,
    },
    /// Removes an existing lobby.
    RemoveLobby {
        lobby_id: Uuid,
    },
    /// Tells a player that the lobby he is trying to connect to is already
    /// full.
    LobbyFull {
        player_tx: UnboundedSender<BackendMessage>,
    },
    /// Broadcasts the current amount of connected clients and players to
    /// clients and players.
    SendConnectionCounts,
    /// Adds a new client.
    AddClient {
        client_id: Uuid,
        client_tx: UnboundedSender<BackendMessage>,
    },
    /// Removes an existing client.
    RemoveClient {
        client_id: Uuid,
    },
}

/// # Handle app message
///
/// Manages the app based on received `AppMessage`. The whole app state is
/// handled in here which allows us to avoid the use of `Mutex` entirely.
pub async fn handle_app_message(mut app: App) -> Result<()> {
    while let Some(msg) = app.rx.recv().await {
        match msg {
            AppMessage::ProvideLobbyInformation { tx, join_mode } => {
                let lobby_information = app.get_lobby_information(join_mode)?;
                let _ = tx.send(lobby_information);
            }
            AppMessage::AddPlayerToLobby { lobby_id, player } => {
                if let Some(lobby) = app.lobbies.get_mut(&lobby_id) {
                    lobby.add_player(player, &app.tx)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::RemovePlayer { player } => {
                if let Some(lobby) = app
                    .lobbies
                    .values_mut()
                    .find(|lobby| lobby.players.contains_key(&player.id))
                {
                    lobby.remove_player(player, &app.tx)?;
                } else {
                    error!(
                        "No lobby has player {}. Unable to delete the player.",
                        player.name
                    );
                }
            }
            AppMessage::SendMessage { player, message } => {
                if let Some(lobby) = app
                    .lobbies
                    .values_mut()
                    .find(|lobby| lobby.players.contains_key(&player.id))
                {
                    lobby.send_message(player, message.clone())?;
                } else {
                    error!(
                        "No lobby has player {}. Unable to send message to the rest of the lobby members.",
                        player.name
                    );
                }
            }

            AppMessage::LobbyFull { player_tx } => {
                let message = BackendMessage::LobbyFull;
                player_tx.send(message)?;
            }

            AppMessage::CurrentLobbies { client_id } => {
                if let Some(client) = app.clients.get(&client_id) {
                    let lobbies = app.get_current_lobbies();
                    let message = BackendMessage::CurrentLobbies(lobbies);
                    client.send(message)?;
                } else {
                    error!("Client with ID {} was not found.", client_id);
                }
            }
            AppMessage::SendLobbyListInformation { lobby_id } => {
                app.send_lobby_list_information(lobby_id)?;
            }
            AppMessage::RemoveLobby { lobby_id } => {
                app.remove_lobby(lobby_id)?;
            }

            AppMessage::AddClient {
                client_id,
                client_tx,
            } => {
                app.clients.insert(client_id, client_tx);
                app.tx.send(AppMessage::SendConnectionCounts)?;
                info!(
                    "Added client with ID {}. Client count is {}.",
                    client_id,
                    app.clients.len()
                );
            }
            AppMessage::RemoveClient { client_id } => {
                app.clients.remove(&client_id);
                app.tx.send(AppMessage::SendConnectionCounts)?;
                info!(
                    "Removed client with ID {}. Client count is {}.",
                    client_id,
                    app.clients.len()
                );
            }
            AppMessage::SendConnectionCounts => {
                let clients = app.clients.len();
                let players = app.lobbies.values().map(|lobby| lobby.players.len()).sum();
                let message = BackendMessage::ConnectionCounts { clients, players };

                // Send counts to all clients.
                for client in app.clients.values() {
                    client.send(message.clone())?;
                }

                // Send counts to all players.
                for lobby in app.lobbies.values() {
                    lobby.broadcast(message.clone())?;
                }
            }
        }
    }
    Ok(())
}
