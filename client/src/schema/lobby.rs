use std::collections::BTreeMap;

use anyhow::Result;
use common::{BackendMessage, ClientMessage, Player};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{debug, error, info};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::{
    chat::Chat,
    encryption::{Encryption, EncryptionAction},
    join::JoinMode,
};
use crate::app::AppMessage;

#[derive(Debug)]
pub enum LobbyMessage {
    CloseConnection,
    CurrentPlayers(BTreeMap<Uuid, Player>),
    PlayerJoined(Player),
    PlayerLeft(Uuid),
    ReceiveMessage(String),
    SendMessage { message: String },
    SetLobbyName { name: String },
    SetLocalPlayerId { id: Uuid },
}

pub struct Lobby {
    pub name: Option<String>,
    pub players: BTreeMap<Uuid, Player>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub chat: Chat,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<LobbyMessage>,
}

impl Lobby {
    /// # Create new lobby connection
    ///
    /// Connects the player to the backend. Depending on `mode` creates or joins a lobby.
    pub async fn new(app_tx: UnboundedSender<AppMessage>, mode: JoinMode) -> Result<Self> {
        // Connect to lobby with given join mode.
        let mut url = String::from("ws://127.0.0.1:3030/play");
        match mode {
            JoinMode::Quickplay => {
                url.push_str("/quickplay");
            }
            JoinMode::Join(lobby_id) => {
                url.push_str(&format!("/join/{}", lobby_id));
            }
            JoinMode::Create => {
                url.push_str("/create");
            }
        };
        let (ws_stream, _) = connect_async(url).await?;

        // Setup messaging channels.
        let (ws_tx, ws_rx) = ws_stream.split();
        let (tx, rx) = unbounded_channel();

        // Spawn task to handle incoming backend messages.
        let message_tx = tx.clone();
        tokio::spawn(Lobby::handle_backend_message(ws_rx, message_tx, app_tx));

        Ok(Self {
            name: None,
            players: BTreeMap::new(),
            encryptions: BTreeMap::new(),
            chat: Chat::new(tx),
            ws_tx,
            rx,
        })
    }

    pub async fn handle_message(&mut self, msg: LobbyMessage) -> Result<()> {
        debug!("Handle message {:?}.", msg);

        match msg {
            LobbyMessage::CloseConnection => {
                info!("Close connection to lobby.");
                self.ws_tx.close().await?;
            }
            LobbyMessage::CurrentPlayers(players) => {
                for (id, player) in players.iter() {
                    let encryption = Encryption {
                        action: EncryptionAction::Joined,
                        index: 0,
                        value: player.name.clone(),
                    };
                    self.encryptions.insert(*id, encryption);
                }
                self.players = players;
            }
            LobbyMessage::PlayerJoined(player) => {
                info!("Player {} joined the lobby.", player.name);

                self.chat.add_message(format!("{} joined!", player.name));
                let encryption = Encryption {
                    action: EncryptionAction::Joined,
                    index: 0,
                    value: player.name.clone(),
                };
                self.encryptions.insert(player.id, encryption);
                self.players.insert(player.id, player);
            }
            LobbyMessage::PlayerLeft(id) => {
                if let Some(player) = self.players.remove(&id) {
                    info!("Player {} left the lobby.", player.name);
                    self.chat.add_message(format!("{} left!", player.name));
                } else {
                    error!("Tried to remove a non-existent player with ID {}.", id);
                }

                if let Some(encryption) = self.encryptions.get_mut(&id) {
                    encryption.index = encryption.value.len() - 1;
                    encryption.action = EncryptionAction::Left;
                }
            }
            LobbyMessage::ReceiveMessage(msg) => {
                self.chat.add_message(msg);
            }
            LobbyMessage::SendMessage { message } => {
                self.ws_tx
                    .send(ClientMessage::SendMessage { message }.into())
                    .await?;
            }
            LobbyMessage::SetLobbyName { name } => {
                debug!("Received lobby name {} from the backend.", name);
                self.name = Some(name);
            }
            LobbyMessage::SetLocalPlayerId { id } => {
                info!("Received clients player ID {} from the backend.", id);

                if let Some(local_player) = self.encryptions.get_mut(&id) {
                    local_player.value.push_str(" (you)");
                }
            }
        }
        Ok(())
    }

    pub async fn handle_backend_message(
        mut ws_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        message_tx: UnboundedSender<LobbyMessage>,
        app_tx: UnboundedSender<AppMessage>,
    ) -> Result<()> {
        while let Some(Ok(msg)) = ws_rx.next().await {
            debug!("Handle backend message {:?}.", msg);

            if msg.is_close() {
                return Ok(());
            }
            let backend_message: BackendMessage = msg.into();
            match backend_message {
                BackendMessage::ProvideLobbyName { name } => {
                    message_tx.send(LobbyMessage::SetLobbyName { name })?;
                }
                BackendMessage::ProvidePlayerId { id } => {
                    message_tx.send(LobbyMessage::SetLocalPlayerId { id })?;
                }
                BackendMessage::CurrentPlayers(players) => {
                    message_tx.send(LobbyMessage::CurrentPlayers(players))?;
                }
                BackendMessage::CloseConnection => {
                    message_tx.send(LobbyMessage::CloseConnection)?;
                }
                BackendMessage::SendMessage(msg) => {
                    message_tx.send(LobbyMessage::ReceiveMessage(msg))?;
                }
                BackendMessage::AddPlayer(player) => {
                    message_tx.send(LobbyMessage::PlayerJoined(player))?;
                }
                BackendMessage::RemovePlayer(player_id) => {
                    message_tx.send(LobbyMessage::PlayerLeft(player_id))?;
                }
                BackendMessage::LobbyFull => {
                    app_tx.send(AppMessage::DisconnectLobby)?;
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
        app_tx.send(AppMessage::DisconnectLobby)?;
        Ok(())
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
