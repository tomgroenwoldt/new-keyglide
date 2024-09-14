use std::collections::BTreeMap;

use fake::{faker::company::en::CompanyName, Fake};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};
use uuid::Uuid;

use common::{
    constants::MAX_LOBBY_SIZE, BackendMessage, ChallengeFiles, LobbyInformation, LobbyListItem,
    LobbyStatus,
};

use crate::{app::message::AppMessage, constants::EMPTY_LOBBY_LIFETIME, player::Player};

#[derive(Clone, Debug)]
pub struct Lobby {
    pub id: Uuid,
    pub name: String,
    /// The current owner of the lobby. It's not guaranteed that there always is
    /// an owner (e.g., in an empty lobby). The first player joining the lobby
    /// is assigned the owner role. If this player leaves the next available
    /// player is assigned.
    pub owner: Option<Uuid>,
    pub players: BTreeMap<Uuid, Player>,
    pub challenge_files: ChallengeFiles,
    pub status: LobbyStatus,
}

impl Default for Lobby {
    fn default() -> Self {
        let start_file =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/start.rs")).to_vec();
        let goal_file =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/goal.rs")).to_vec();

        let challenge_files = ChallengeFiles {
            start_file,
            goal_file,
        };
        let id = Uuid::new_v4();
        Self {
            id,
            name: CompanyName().fake(),
            owner: None,
            players: BTreeMap::new(),
            challenge_files,
            status: LobbyStatus::WaitingForPlayers,
        }
    }
}

impl Lobby {
    /// # Broadcast message
    ///
    /// Sends a message to every player inside the lobby.
    pub fn broadcast(&self, msg: BackendMessage) {
        for Player {
            id: _,
            name: _,
            tx,
            progress: _,
        } in self.players.values()
        {
            let _ = tx.send(msg.clone());
        }
    }

    pub fn to_list_item(&self) -> LobbyListItem {
        LobbyListItem {
            name: self.name.clone(),
            player_count: self.players.len(),
            status: self.status.clone(),
        }
    }

    pub fn to_information(&self) -> LobbyInformation {
        let mut players = BTreeMap::new();
        for (id, player) in self.players.iter() {
            players.insert(*id, player.to_common_player());
        }
        LobbyInformation {
            id: self.id,
            name: self.name.clone(),
            status: self.status.clone(),
            owner: self.owner,
            players,
            challenge_files: self.challenge_files.clone(),
        }
    }

    /// # Add player
    ///
    /// Adds a player to the lobby. If the lobby is full, tell the player about
    /// that and prevent the addition. If the player successfully joined the
    /// lobby tell him the lobby name.
    pub fn add_player(&mut self, player: Player, app_tx: &UnboundedSender<AppMessage>) {
        // Return early if the lobby is full.
        if self.players.len() >= MAX_LOBBY_SIZE {
            warn!(
                "Tried to add player {} to full lobby {}.",
                player.name, self.name
            );
            let _ = app_tx.send(AppMessage::LobbyFull {
                player_tx: player.tx,
            });
            return;
        }

        if self.status != LobbyStatus::WaitingForPlayers {
            warn!(
                "Tried to add player {} to lobby {} but it's not waiting for players.",
                player.name, self.name
            );
            let _ = app_tx.send(AppMessage::LobbyNotWaitingForPlayers {
                player_tx: player.tx,
            });
            return;
        }

        // Insert the player into the player map.
        self.players.insert(player.id, player.clone());
        info!("Added player {} to lobby {}.", player.name, self.name);

        // Tell connected players about this new player.
        let message = BackendMessage::AddPlayer(player.to_common_player());
        self.broadcast(message);

        // Tell non-playing clients about the new player taking up a seat in
        // this lobby.
        let _ = app_tx.send(AppMessage::SendLobbyPlayerCountUpdate { lobby_id: self.id });

        // Tell everyone about the update in connections.
        let _ = app_tx.send(AppMessage::SendConnectionCounts);

        // If the new player is the only player in the lobby, assign the owner
        // role.
        if self.players.len() == 1 {
            self.owner = Some(player.id);

            // Tell the new player that he's the owner.
            let _ = player
                .tx
                .send(BackendMessage::AssignOwner { id: player.id });
        }

        // Tell the player about his own ID.
        let _ = player
            .tx
            .send(BackendMessage::ProvidePlayerId { id: player.id });
    }

    /// # Remove player
    ///
    /// Removes a player from the lobby if he exists.
    pub fn remove_player(&mut self, player: Player, app_tx: &UnboundedSender<AppMessage>) {
        if let Some(player) = self.players.remove(&player.id) {
            info!("Removed player {} from lobby {}.", player.name, self.name);
            // Tell connected players about the removal of this player.
            let message = BackendMessage::RemovePlayer(player.id);
            self.broadcast(message);

            // Tell connected players about the removal of the lobby owner and
            // the new assignee.
            if self.owner.is_some_and(|owner_id| owner_id.eq(&player.id)) {
                if let Some((player_id, _)) = self.players.first_key_value() {
                    self.owner = Some(*player_id);
                    let message = BackendMessage::AssignOwner { id: *player_id };
                    self.broadcast(message);
                }
            }

            // Tell non-playing clients about the free seat in this lobby.
            let _ = app_tx.send(AppMessage::SendLobbyPlayerCountUpdate { lobby_id: self.id });

            // Tell everyone about the update in connections.
            let _ = app_tx.send(AppMessage::SendConnectionCounts);

            // Now, if the lobby is empty, tell the app to remove this lobby.
            if self.players.is_empty() {
                let app_tx = app_tx.clone();
                let lobby_id = self.id;

                // Remove the owner, as there are no players in the lobby.
                self.owner = None;
                // Also, reset the status and tell the clients about it.
                self.status = LobbyStatus::WaitingForPlayers;
                let _ = app_tx.send(AppMessage::SendLobbyStatusUpdate { lobby_id: self.id });

                // Tell the app to remove the lobby after 30 seconds.
                tokio::spawn(async move {
                    tokio::time::sleep(EMPTY_LOBBY_LIFETIME).await;
                    let _ = app_tx.send(AppMessage::RemoveLobby { lobby_id });
                });
            }
        } else {
            error!(
                "Player {} was not found in lobby {}.",
                player.name, self.name
            );
        }
    }

    /// # Send message
    ///
    /// Broadcasts a message from a player to all connnected players if the
    /// player exists.
    pub fn send_message(&self, player: Player, message: String) {
        if let Some(player) = self.players.get(&player.id) {
            let message = BackendMessage::SendMessage(format!("{}: {message}", player.name));
            self.broadcast(message);
        } else {
            error!(
                "Player {} was not found in lobby {}.",
                player.name, self.name
            );
        }
    }
}
