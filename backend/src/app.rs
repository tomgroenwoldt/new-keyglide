use std::collections::BTreeMap;

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::info;
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage, Player};

use crate::lobby::Lobby;

use self::message::AppMessage;

pub(crate) mod message;

#[derive(Debug)]
pub struct App {
    /// All connected clients that are not in a lobby.
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

    /// # Add player via quickplay
    ///
    /// Adds the player to the nonfull lobby with the most players. Creates a new lobby if no
    /// nonfull lobby was found.
    pub fn add_player_via_quickplay(&mut self, player: Player) -> Result<()> {
        if let Some(lobby) = self
            .lobbies
            .values_mut()
            .filter(|lobby| lobby.players.len() < MAX_LOBBY_SIZE)
            .max_by_key(|lobby| lobby.players.len())
        {
            lobby.add_player(player, &self.tx)?;
        } else {
            self.add_player_to_new_lobby(player)?;
        }
        Ok(())
    }

    /// # Add player to new lobby
    ///
    /// Creates a new lobby and inserts the given player into it.
    pub fn add_player_to_new_lobby(&mut self, player: Player) -> Result<()> {
        // Create the new lobby.
        let lobby = Lobby::default();
        self.lobbies.insert(lobby.id, lobby.clone());
        info!(
            "Created new lobby {}. {} open lobby/lobbies.",
            lobby.name,
            self.lobbies.len()
        );

        // Insert the player.
        if let Some(lobby) = self.lobbies.get_mut(&lobby.id) {
            lobby.add_player(player, &self.tx)?;
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
        if let Some(lobby) = self.lobbies.get(&lobby_id) {
            if lobby.players.is_empty() {
                if let Some(lobby) = self.lobbies.remove(&lobby_id) {
                    info!(
                        "Removed lobby {} with {} remaning players. {} lobby/lobbies remain.",
                        lobby.name,
                        lobby.players.len(),
                        self.lobbies.len(),
                    );
                    for client in self.clients.values() {
                        client.send(BackendMessage::RemoveLobby(lobby_id))?;
                    }
                }
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
