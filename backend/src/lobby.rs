use std::collections::BTreeMap;

use anyhow::Result;
use fake::{faker::company::en::CompanyName, Fake};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage, Player};

use crate::app::AppMessage;

#[derive(Debug)]
pub struct Lobby {
    pub id: Uuid,
    pub name: String,
    /// All connected players for this lobby.
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
        for Player { name: _, tx } in self.players.values() {
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
    /// that and prevent the addition.
    pub fn add_player(
        &mut self,
        player_id: Uuid,
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

        info!("Added player {} to lobby {}.", player.name, self.name);
        // Insert the player into the player map.
        self.players.insert(player_id, player.clone());

        // Tell connected players about this new player.
        let message = BackendMessage::AddPlayer(player_id, player);
        self.broadcast(message)?;

        // Tell non-connected clients about the new player taking up a seat in
        // this lobby.
        app_tx.send(AppMessage::SendLobbyInformation { lobby_id: self.id })?;

        // Tell the new player about all current players.
        app_tx.send(AppMessage::CurrentPlayers {
            lobby_id: self.id,
            player_id,
        })?;
        Ok(())
    }

    /// # Remove player
    ///
    /// Removes a player from the lobby if he exists.
    pub fn remove_player(
        &mut self,
        player_id: Uuid,
        app_tx: &UnboundedSender<AppMessage>,
    ) -> Result<()> {
        if let Some(player) = self.players.remove(&player_id) {
            info!("Removed player {} from lobby {}.", player.name, self.name);
            // Tell connected players about the removal of this player.
            let message = BackendMessage::RemovePlayer(player_id);
            self.broadcast(message)?;

            // Now, if the lobby is empty, tell the app to remove this lobby.
            // Otherwise, tell non-connected clients about the free seat in
            // this lobby.
            if self.players.is_empty() {
                app_tx.send(AppMessage::RemoveLobby { lobby_id: self.id })?;
            } else {
                app_tx.send(AppMessage::SendLobbyInformation { lobby_id: self.id })?;
            }
        } else {
            error!("Player with ID {} was not found.", player_id);
        }
        Ok(())
    }

    /// # Send message
    ///
    /// Broadcasts a message from a player to all connnected players if the
    /// player exists.
    pub fn send_message(&self, player_id: Uuid, message: String) -> Result<()> {
        if let Some(player) = self.players.get(&player_id) {
            let message = BackendMessage::SendMessage(format!("{}: {message}", player.name));
            self.broadcast(message)?;
        } else {
            error!("Player with ID {} was not found.", player_id);
        }
        Ok(())
    }

    /// # Send current players
    ///
    /// Sends all already connected players to the specified player.
    pub fn send_current_players(&self, player_id: Uuid) -> Result<()> {
        if let Some(player) = self.players.get(&player_id) {
            player
                .tx
                .send(BackendMessage::CurrentPlayers(self.players.clone()))?;
        } else {
            error!("Player with ID {} was not found.", player_id);
        }
        Ok(())
    }
}
