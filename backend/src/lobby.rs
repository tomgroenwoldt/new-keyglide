use std::collections::BTreeMap;

use anyhow::Result;
use fake::{faker::company::en::CompanyName, Fake};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use common::{
    constants::MAX_LOBBY_SIZE, BackendMessage, ChallengeFiles, LobbyInformation, LobbyListItem,
};

use crate::{app::message::AppMessage, constants::EMPTY_LOBBY_LIFETIME, player::Player};

#[derive(Clone, Debug)]
pub struct Lobby {
    pub id: Uuid,
    pub name: String,
    pub players: BTreeMap<Uuid, Player>,
    pub challenge_files: ChallengeFiles,
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
        debug!("Lobby ID: {}", id);
        Self {
            id,
            name: CompanyName().fake(),
            players: BTreeMap::new(),
            challenge_files,
        }
    }
}

impl Lobby {
    /// # Broadcast message
    ///
    /// Sends a message to every player inside the lobby.
    pub fn broadcast(&self, msg: BackendMessage) -> Result<()> {
        for Player { id: _, name: _, tx } in self.players.values() {
            tx.send(msg.clone())?;
        }
        Ok(())
    }

    pub fn to_list_item(&self) -> LobbyListItem {
        LobbyListItem {
            name: self.name.clone(),
            player_count: self.players.len(),
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
            players,
            challenge_files: self.challenge_files.clone(),
        }
    }

    /// # Add player
    ///
    /// Adds a player to the lobby. If the lobby is full, tell the player about
    /// that and prevent the addition. If the player successfully joined the
    /// lobby tell him the lobby name.
    pub fn add_player(
        &mut self,
        player: Player,
        app_tx: &UnboundedSender<AppMessage>,
    ) -> Result<()> {
        // Return early if the lobby is full.
        if self.players.len() >= MAX_LOBBY_SIZE {
            warn!(
                "Tried to add player {} to full lobby {}.",
                player.name, self.name
            );
            app_tx.send(AppMessage::LobbyFull {
                player_tx: player.tx,
            })?;
            return Ok(());
        }

        // Insert the player into the player map.
        self.players.insert(player.id, player.clone());
        info!("Added player {} to lobby {}.", player.name, self.name);

        // Tell connected players about this new player.
        let message = BackendMessage::AddPlayer(player.to_common_player());
        self.broadcast(message)?;

        // Tell non-playing clients about the new player taking up a seat in
        // this lobby.
        app_tx.send(AppMessage::SendLobbyListInformation { lobby_id: self.id })?;

        // Tell everyone about the update in connections.
        app_tx.send(AppMessage::SendConnectionCounts)?;

        // Tell the player about his own ID.
        player
            .tx
            .send(BackendMessage::ProvidePlayerId { id: player.id })?;

        Ok(())
    }

    /// # Remove player
    ///
    /// Removes a player from the lobby if he exists.
    pub fn remove_player(
        &mut self,
        player: Player,
        app_tx: &UnboundedSender<AppMessage>,
    ) -> Result<()> {
        if let Some(player) = self.players.remove(&player.id) {
            info!("Removed player {} from lobby {}.", player.name, self.name);
            // Tell connected players about the removal of this player.
            let message = BackendMessage::RemovePlayer(player.id);
            self.broadcast(message)?;

            // Tell non-playing clients about the free seat in this lobby.
            app_tx.send(AppMessage::SendLobbyListInformation { lobby_id: self.id })?;

            // Tell everyone about the update in connections.
            app_tx.send(AppMessage::SendConnectionCounts)?;

            // Now, if the lobby is empty, tell the app to remove this lobby.
            if self.players.is_empty() {
                let app_tx = app_tx.clone();
                let lobby_id = self.id;

                // Tell the app to remove the lobby after 30 seconds.
                tokio::spawn(async move {
                    tokio::time::sleep(EMPTY_LOBBY_LIFETIME).await;
                    if let Err(e) = app_tx.send(AppMessage::RemoveLobby { lobby_id }) {
                        error!("Error sending via app channel: {e}");
                    }
                });
            }
        } else {
            error!(
                "Player {} was not found in lobby {}.",
                player.name, self.name
            );
        }
        Ok(())
    }

    /// # Send message
    ///
    /// Broadcasts a message from a player to all connnected players if the
    /// player exists.
    pub fn send_message(&self, player: Player, message: String) -> Result<()> {
        if let Some(player) = self.players.get(&player.id) {
            let message = BackendMessage::SendMessage(format!("{}: {message}", player.name));
            self.broadcast(message)?;
        } else {
            error!(
                "Player {} was not found in lobby {}.",
                player.name, self.name
            );
        }
        Ok(())
    }
}
