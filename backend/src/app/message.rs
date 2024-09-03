use anyhow::Result;
use chrono::Utc;
use tokio::sync::{mpsc::UnboundedSender, oneshot::Sender};
use tracing::{error, info, warn};
use uuid::Uuid;

use common::{BackendMessage, JoinMode, LobbyInformation, LobbyStatus};

use super::App;
use crate::{
    constants::{LOBBY_FINISH_TIME, LOBBY_START_TIMER, MAX_LOBBY_PLAY_TIME},
    player::Player,
};

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
        lobby_id: Uuid,
    },
    /// Broadcasts a message of provided player to all connected players.
    SendMessage {
        player: Player,
        message: String,
        lobby_id: Uuid,
    },

    /// Broadcasts all existing lobbies to a freshly connected client.
    CurrentLobbies {
        client_id: Uuid,
    },
    /// Broadcasts name, player count, and status of a lobby to all connected
    /// clients.
    AddLobby {
        lobby_id: Uuid,
    },
    /// Broadcasts the new lobby player counts to all connected clients.
    SendLobbyPlayerCountUpdate {
        lobby_id: Uuid,
    },
    SendLobbyStatusUpdate {
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
    /// Tells a player that the lobby he is trying to connect to is not
    /// waiting for any players.
    LobbyNotWaitingForPlayers {
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
    /// Requests to start the game inside a lobby if the provided player is the
    /// lobby owner.
    RequestStart {
        player: Player,
        lobby_id: Uuid,
    },
    /// Starts the game inside a lobby.
    Start {
        lobby_id: Uuid,
    },
    /// Finishes the game inside a lobby.
    Finish {
        lobby_id: Uuid,
    },
    /// Resets the game inside a lobby.
    Reset {
        lobby_id: Uuid,
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
                let lobby_id = app.get_lobby_id(join_mode)?;
                let Some(lobby) = app.lobbies.get(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                let lobby_information = lobby.to_information();
                let _ = tx.send(lobby_information);
            }
            AppMessage::AddPlayerToLobby { lobby_id, player } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                lobby.add_player(player, &app.tx)?;
            }
            AppMessage::RemovePlayer { player, lobby_id } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                lobby.remove_player(player, &app.tx)?;
            }
            AppMessage::SendMessage {
                player,
                message,
                lobby_id,
            } => {
                let Some(lobby) = app.lobbies.get(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                lobby.send_message(player, message.clone())?;
            }

            AppMessage::LobbyFull { player_tx } => {
                let message = BackendMessage::LobbyFull;
                player_tx.send(message)?;
            }

            AppMessage::CurrentLobbies { client_id } => {
                let Some(client) = app.clients.get(&client_id) else {
                    error!("Client with ID {} was not found.", client_id);
                    continue;
                };
                let lobbies = app.get_current_lobbies();
                let message = BackendMessage::CurrentLobbies(lobbies);
                client.send(message)?;
            }
            AppMessage::AddLobby { lobby_id } => {
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
            AppMessage::RequestStart { player, lobby_id } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                // Start the game inside the lobby if the player is the
                // lobby owner.
                if lobby.owner.is_some_and(|owner_id| owner_id.eq(&player.id))
                    && lobby.status == LobbyStatus::WaitingForPlayers
                {
                    // Change the lobby status and tell clients about it.
                    lobby.status = LobbyStatus::AboutToStart(Utc::now() + LOBBY_START_TIMER);
                    app.tx
                        .send(AppMessage::SendLobbyStatusUpdate { lobby_id: lobby.id })?;
                    // Tell players in the lobby about the status update.
                    lobby.broadcast(BackendMessage::StatusUpdate {
                        status: lobby.status.clone(),
                    })?;

                    // Wait for a duration of `LOBBY_START_TIMER` and tell
                    // the application to start the lobby.
                    let app_tx = app.tx.clone();
                    let lobby_id = lobby.id;
                    tokio::spawn(async move {
                        tokio::time::sleep(LOBBY_START_TIMER).await;
                        if let Err(e) = app_tx.send(AppMessage::Start { lobby_id }) {
                            error!("Error sending via app channel: {e}");
                        }
                    });
                }
            }
            AppMessage::Start { lobby_id } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                let LobbyStatus::AboutToStart(_) = lobby.status else {
                    warn!(
                        "Tried to start lobby {} with {} players that was not about to start.",
                        lobby.name,
                        lobby.players.len()
                    );
                    continue;
                };
                lobby.status = LobbyStatus::InProgress(Utc::now() + MAX_LOBBY_PLAY_TIME);
                // Tell clients about the started lobby.
                app.tx
                    .send(AppMessage::SendLobbyStatusUpdate { lobby_id: lobby.id })?;
                // Tell players in the lobby about the status update.
                lobby.broadcast(BackendMessage::StatusUpdate {
                    status: lobby.status.clone(),
                })?;

                // Put the lobby in `LobbyStatus::Finish` after two minutes.
                let app_tx = app.tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(MAX_LOBBY_PLAY_TIME).await;
                    if let Err(e) = app_tx.send(AppMessage::Finish { lobby_id }) {
                        error!("Error sending via app channel: {e}");
                    }
                });
            }
            AppMessage::LobbyNotWaitingForPlayers { player_tx } => {
                let message = BackendMessage::LobbyNotWaitingForPlayers;
                player_tx.send(message)?;
            }
            AppMessage::SendLobbyPlayerCountUpdate { lobby_id } => {
                let Some(lobby) = app.lobbies.get(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                for client in app.clients.values() {
                    client.send(BackendMessage::UpdateLobbyPlayerCount {
                        id: lobby_id,
                        player_count: lobby.players.len(),
                    })?;
                }
            }
            AppMessage::SendLobbyStatusUpdate { lobby_id } => {
                let Some(lobby) = app.lobbies.get(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                for client in app.clients.values() {
                    client.send(BackendMessage::UpdateLobbyStatus {
                        id: lobby_id,
                        status: lobby.status.clone(),
                    })?;
                }
            }
            AppMessage::Finish { lobby_id } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                lobby.status = LobbyStatus::Finish(Utc::now() + LOBBY_FINISH_TIME);
                // Tell clients about the finished lobby.
                app.tx
                    .send(AppMessage::SendLobbyStatusUpdate { lobby_id: lobby.id })?;
                // Tell players in the lobby about the status update.
                lobby.broadcast(BackendMessage::StatusUpdate {
                    status: lobby.status.clone(),
                })?;

                // Put the lobby in `LobbyStatus::WaitingForPlayers` after two minutes.
                let app_tx = app.tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(LOBBY_FINISH_TIME).await;
                    if let Err(e) = app_tx.send(AppMessage::Reset { lobby_id }) {
                        error!("Error sending via app channel: {e}");
                    }
                });
            }
            AppMessage::Reset { lobby_id } => {
                let Some(lobby) = app.lobbies.get_mut(&lobby_id) else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                    continue;
                };
                lobby.status = LobbyStatus::WaitingForPlayers;
                // Tell clients about the reset lobby.
                app.tx
                    .send(AppMessage::SendLobbyStatusUpdate { lobby_id: lobby.id })?;
                // Tell players in the lobby about the status update.
                lobby.broadcast(BackendMessage::StatusUpdate {
                    status: lobby.status.clone(),
                })?;
            }
        }
    }
    Ok(())
}
