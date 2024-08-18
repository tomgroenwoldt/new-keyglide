use std::collections::BTreeMap;

use anyhow::Result;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{debug, error, info};
use ratatui::crossterm::event::KeyEvent;
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, Message},
    MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage};

use super::encryption::{Encryption, EncryptionAction};
use crate::{app::AppMessage, config::Config};

pub struct Join {
    pub lobbies: BTreeMap<Uuid, common::Lobby>,
    pub selected_lobby: Option<Uuid>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<JoinMessage>,
    pub app_tx: UnboundedSender<AppMessage>,
}

#[derive(Debug)]
pub enum JoinMessage {
    CurrentLobbies(BTreeMap<Uuid, common::Lobby>),
    CloseConnection,
    AddLobby(Uuid, common::Lobby),
    RemoveLobby(Uuid),
}

#[derive(Debug)]
pub enum JoinMode {
    Quickplay,
    Join(Uuid),
    Create,
}

impl Join {
    pub async fn new(app_tx: UnboundedSender<AppMessage>) -> Result<Self, Error> {
        let (ws_stream, _) = connect_async("ws://127.0.0.1:3030/lobbies").await?;
        let (ws_tx, ws_rx) = ws_stream.split();

        let (tx, rx) = unbounded_channel();
        let message_tx = tx.clone();
        tokio::spawn(Join::handle_backend_message(
            ws_rx,
            message_tx,
            app_tx.clone(),
        ));

        Ok(Self {
            lobbies: BTreeMap::new(),
            selected_lobby: None,
            encryptions: BTreeMap::new(),
            ws_tx,
            rx,
            app_tx,
        })
    }

    pub async fn handle_key_event(&mut self, config: &Config, key: KeyEvent) -> Result<()> {
        debug!("Handle key event {:?}.", key);

        // Join a selected lobby.
        if key.eq(&config.key_bindings.join.join_selected) {
            if let Some(selected_lobby) = self.selected_lobby {
                self.ws_tx.close().await?;
                let join_mode = JoinMode::Join(selected_lobby);
                self.app_tx.send(AppMessage::ConnectToLobby { join_mode })?;
            }
        } else if key.eq(&config.key_bindings.movement.down) {
            self.next_lobby_entry();
        } else if key.eq(&config.key_bindings.movement.up) {
            self.previous_lobby_entry();
        } else if key.eq(&config.key_bindings.join.quickplay) {
            self.ws_tx.close().await?;
            let join_mode = JoinMode::Quickplay;
            self.app_tx.send(AppMessage::ConnectToLobby { join_mode })?;
        } else if key.eq(&config.key_bindings.join.create) {
            debug!("Close client connection.");

            self.ws_tx.close().await?;
            let join_mode = JoinMode::Create;
            self.app_tx.send(AppMessage::ConnectToLobby { join_mode })?;
        }
        Ok(())
    }

    pub async fn handle_message(&mut self, msg: JoinMessage) -> Result<()> {
        debug!("Handle message {:?}.", msg);

        match msg {
            JoinMessage::CurrentLobbies(lobbies) => {
                for (id, lobby) in lobbies.iter() {
                    let value =
                        format!("{} ({}/{})", lobby.name, lobby.player_count, MAX_LOBBY_SIZE);
                    let encryption = Encryption {
                        action: EncryptionAction::Joined,
                        index: 0,
                        value,
                    };
                    self.encryptions.insert(*id, encryption);
                }
                self.lobbies = lobbies;
            }
            JoinMessage::CloseConnection => {
                info!("Close non-player connection.");
                self.ws_tx.close().await?;
            }
            JoinMessage::AddLobby(lobby_id, lobby) => {
                info!(
                    "Update lobby list with lobby {} and {} players.",
                    lobby.name, lobby.player_count
                );

                let value = format!("{} ({}/{})", lobby.name, lobby.player_count, MAX_LOBBY_SIZE);
                let encryption = Encryption {
                    action: EncryptionAction::Joined,
                    index: 0,
                    value,
                };
                self.encryptions.insert(lobby_id, encryption);
                self.lobbies.insert(lobby_id, lobby);
            }
            JoinMessage::RemoveLobby(lobby_id) => {
                if let Some(encryption) = self.encryptions.get_mut(&lobby_id) {
                    encryption.index = encryption.value.len() - 1;
                    encryption.action = EncryptionAction::Left;
                }
                // If the currently selected lobby was removed, unselect it.
                if let Some(selected_lobby) = self.selected_lobby {
                    if selected_lobby.eq(&lobby_id) {
                        self.selected_lobby = None;
                    }
                }
                if let Some(lobby) = self.lobbies.remove(&lobby_id) {
                    info!("Remove lobby {} from lobby list.", lobby.name);
                } else {
                    error!("Tried to remove a non-existent lobby with ID {}.", lobby_id);
                }
            }
        }
        Ok(())
    }

    pub async fn handle_backend_message(
        mut ws_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        message_tx: UnboundedSender<JoinMessage>,
        app_tx: UnboundedSender<AppMessage>,
    ) -> Result<()> {
        while let Some(Ok(msg)) = ws_rx.next().await {
            debug!("Handle backend message {:?}.", msg);

            if msg.is_close() {
                return Ok(());
            }
            let backend_message: BackendMessage = msg.into();
            match backend_message {
                BackendMessage::CloseConnection => {
                    message_tx.send(JoinMessage::CloseConnection)?;
                    return Ok(());
                }
                BackendMessage::CurrentLobbies(lobbies) => {
                    message_tx.send(JoinMessage::CurrentLobbies(lobbies))?;
                }
                BackendMessage::AddLobby(lobby_id, lobby) => {
                    message_tx.send(JoinMessage::AddLobby(lobby_id, lobby))?;
                }
                BackendMessage::RemoveLobby(lobby_id) => {
                    message_tx.send(JoinMessage::RemoveLobby(lobby_id))?;
                }
                BackendMessage::ConnectionCounts { clients, players } => {
                    app_tx.send(AppMessage::ConnectionCounts { clients, players })?;
                }
                _ => {}
            }
        }

        // We should only arrive here whenever the WS connection is abruptly
        // closed. Therefore remove the current lobby here.
        error!("Backend service disconnected!");
        app_tx.send(AppMessage::ServiceDisconnected)?;
        Ok(())
    }

    /// # Next lobby entry
    ///
    /// Selects the next lobby entry given an already selected lobby. Otherwise
    /// select the first entry.
    pub fn next_lobby_entry(&mut self) {
        let next_lobby_id = if let Some(lobby_id) = self.selected_lobby {
            self.lobbies
                .range(lobby_id..)
                .nth(1)
                .or_else(|| self.lobbies.range(..=lobby_id).next())
                .map(|(id, _)| *id)
        } else {
            self.lobbies
                .first_key_value()
                .map(|(lobby_id, _)| *lobby_id)
        };
        debug!(
            "Switch from lobby {:?} to next lobby {:?}.",
            self.selected_lobby, next_lobby_id
        );
        self.selected_lobby = next_lobby_id;
    }

    /// # Previous lobby entry
    ///
    /// Selects the previous lobby entry given an already selected lobby. Otherwise
    /// select the last entry.
    pub fn previous_lobby_entry(&mut self) {
        let previous_lobby_id = if let Some(lobby_id) = self.selected_lobby {
            self.lobbies
                .range(..lobby_id)
                .next_back()
                .or_else(|| self.lobbies.iter().next_back())
                .map(|(lobby_id, _)| *lobby_id)
        } else {
            self.lobbies.last_key_value().map(|(lobby_id, _)| *lobby_id)
        };
        debug!(
            "Switch from lobby {:?} to previous lobby {:?}.",
            self.selected_lobby, previous_lobby_id
        );
        self.selected_lobby = previous_lobby_id;
    }

    pub fn on_tick(&mut self) {
        let mut encryptions_to_delete = vec![];

        for (id, encryption) in self.encryptions.iter_mut() {
            match encryption.action {
                EncryptionAction::Joined => {
                    if encryption.index < encryption.value.len() {
                        encryption.index += 1;
                    }
                }
                EncryptionAction::Left => {
                    if encryption.value.pop().is_none() {
                        encryptions_to_delete.push(*id);
                    }
                }
            }
        }
        for id in encryptions_to_delete {
            self.encryptions.remove(&id);
        }
    }
}
