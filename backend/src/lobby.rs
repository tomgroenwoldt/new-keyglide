use std::collections::BTreeMap;

use anyhow::Result;
use fake::{faker::company::en::CompanyName, Fake};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage};

use crate::{app::message::AppMessage, constants::EMPTY_LOBBY_LIFETIME, player::Player};

#[derive(Clone, Debug)]
pub struct Lobby {
    pub id: Uuid,
    pub name: String,
    pub players: BTreeMap<Uuid, Player>,
}

impl Default for Lobby {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: CompanyName().fake(),
            players: BTreeMap::new(),
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

    pub fn to_common_lobby(&self) -> common::Lobby {
        common::Lobby {
            name: self.name.clone(),
            player_count: self.players.len(),
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

        // Tell the player about the lobby name.
        player.tx.send(BackendMessage::ProvideLobbyName {
            name: self.name.clone(),
        })?;

        // Insert the player into the player map.
        self.players.insert(player.id, player.clone());
        info!("Added player {} to lobby {}.", player.name, self.name);

        // Tell connected players about this new player.
        let message = BackendMessage::AddPlayer(player.to_common_player());
        self.broadcast(message)?;

        // Tell non-playing clients about the new player taking up a seat in
        // this lobby and the update in connections.
        app_tx.send(AppMessage::SendLobbyInformation { lobby_id: self.id })?;
        app_tx.send(AppMessage::SendConnectionCounts)?;

        // Tell the new player about all current players.
        app_tx.send(AppMessage::CurrentPlayers {
            lobby_id: self.id,
            player,
        })?;

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

            // Tell non-playing clients about the free sear in this lobby and
            // the update in connections.
            app_tx.send(AppMessage::SendLobbyInformation { lobby_id: self.id })?;
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

    /// # Send current players
    ///
    /// Sends all already connected players to the specified player.
    pub fn send_current_players(&self, player: Player) -> Result<()> {
        if let Some(player) = self.players.get(&player.id) {
            let mut players = BTreeMap::new();
            for player in self.players.values() {
                players.insert(player.id, player.to_common_player());
            }
            player.tx.send(BackendMessage::CurrentPlayers(players))?;

            // Tell the player about his own ID.
            player
                .tx
                .send(BackendMessage::ProvidePlayerId { id: player.id })?;
        } else {
            error!(
                "Player {} was not found in lobby {}.",
                player.name, self.name
            );
        }
        Ok(())
    }
}
