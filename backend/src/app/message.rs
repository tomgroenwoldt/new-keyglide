use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};
use uuid::Uuid;

use common::{BackendMessage, Player};

use super::App;

pub enum AppMessage {
    // Lobby messages.
    CurrentPlayers {
        lobby_id: Uuid,
        player: Player,
    },
    AddPlayerAndCreateLobby {
        player: Player,
    },
    AddPlayerViaQuickplay {
        player: Player,
    },
    AddPlayerToLobby {
        lobby_id: Uuid,
        player: Player,
    },
    SendMessage {
        player: Player,
        message: String,
    },
    RemovePlayer {
        player: Player,
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
/// Manages the app based on received `AppMessage`.
pub async fn handle_app_message(mut app: App) -> Result<()> {
    while let Some(msg) = app.rx.recv().await {
        match msg {
            AppMessage::CurrentPlayers { lobby_id, player } => {
                if let Some(lobby) = app.lobbies.get(&lobby_id) {
                    lobby.send_current_players(player)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::AddPlayerAndCreateLobby { player } => {
                app.add_player_to_new_lobby(player)?;
            }
            AppMessage::AddPlayerViaQuickplay { player } => {
                app.add_player_via_quickplay(player)?;
            }
            AppMessage::AddPlayerToLobby { lobby_id, player } => {
                if let Some(lobby) = app.lobbies.get_mut(&lobby_id) {
                    lobby.add_player(player, &app.tx)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
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
            AppMessage::SendLobbyInformation { lobby_id } => {
                app.send_lobby_information(lobby_id)?;
            }
            AppMessage::RemoveLobby { lobby_id } => {
                app.remove_lobby(lobby_id)?;
            }

            AppMessage::AddClient {
                client_id,
                client_tx,
            } => {
                app.clients.insert(client_id, client_tx);
                info!(
                    "Added client with ID {}. {} client/clients connected.",
                    client_id,
                    app.clients.len()
                );
            }
            AppMessage::RemoveClient { client_id } => {
                app.clients.remove(&client_id);
                info!(
                    "Removed client with ID {}. {} client/clients remain.",
                    client_id,
                    app.clients.len()
                );
            }
        }
    }
    Ok(())
}
