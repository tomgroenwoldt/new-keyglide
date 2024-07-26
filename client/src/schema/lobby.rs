use std::collections::BTreeMap;

use anyhow::Result;
use common::{BackendMessage, ClientMessage, Player};
use fake::{faker::name::raw::Name, locales::EN, Fake};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use urlencoding::encode;
use uuid::Uuid;

use super::chat::Chat;

pub enum LobbyMessage {
    CloseConnection,
    CurrentPlayers(BTreeMap<Uuid, Player>),
    PlayerJoined(Uuid, Player),
    PlayerLeft(Uuid),
    ReceiveMessage(String),
    SendMessage(String),
}

pub struct Lobby {
    pub players: BTreeMap<Uuid, Player>,
    pub username: String,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub chat: Chat,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<LobbyMessage>,
}

pub enum EncryptionAction {
    Joined,
    Left,
}

pub struct Encryption {
    pub action: EncryptionAction,
    pub index: usize,
    pub name: String,
}

impl Lobby {
    pub async fn new() -> Result<Self> {
        let username: String = Name(EN).fake();
        let encoded_name = encode(&username);
        let (ws_stream, _) = connect_async(format!("ws://127.0.0.1:3030/{encoded_name}")).await?;
        let (ws_tx, mut ws_rx) = ws_stream.split();

        let (tx, rx) = unbounded_channel();

        let message_tx = tx.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                let backend_message: BackendMessage = msg.into();
                match backend_message {
                    BackendMessage::CurrentPlayers(players) => {
                        message_tx
                            .send(LobbyMessage::CurrentPlayers(players))
                            .unwrap();
                    }
                    BackendMessage::PlayerJoined(id, player) => {
                        message_tx
                            .send(LobbyMessage::PlayerJoined(id, player))
                            .unwrap();
                    }
                    BackendMessage::PlayerLeft(id) => {
                        message_tx.send(LobbyMessage::PlayerLeft(id)).unwrap();
                    }
                    BackendMessage::CloseConnection => {
                        message_tx.send(LobbyMessage::CloseConnection).unwrap();
                    }
                    BackendMessage::SendMessage(msg) => {
                        message_tx.send(LobbyMessage::ReceiveMessage(msg)).unwrap();
                    }
                    BackendMessage::Unknown => {}
                }
            }
        });

        Ok(Self {
            players: BTreeMap::new(),
            username,
            encryptions: BTreeMap::new(),
            chat: Chat::new(tx),
            ws_tx,
            rx,
        })
    }

    pub async fn handle_message(&mut self, msg: LobbyMessage) -> Result<()> {
        match msg {
            LobbyMessage::CloseConnection => {
                self.ws_tx.close().await?;
            }
            LobbyMessage::CurrentPlayers(players) => {
                for (id, player) in players.iter() {
                    let encryption = Encryption {
                        action: EncryptionAction::Joined,
                        index: 0,
                        name: player.name.clone(),
                    };
                    self.encryptions.insert(*id, encryption);
                }
                self.players = players;
            }
            LobbyMessage::PlayerJoined(id, player) => {
                let joined_player = if player.name.eq(&self.username) {
                    "You"
                } else {
                    &player.name
                };
                self.chat.add_message(format!("{} joined!", joined_player));
                let encryption = Encryption {
                    action: EncryptionAction::Joined,
                    index: 0,
                    name: player.name.clone(),
                };
                self.encryptions.insert(id, encryption);
                self.players.insert(id, player);
            }
            LobbyMessage::PlayerLeft(id) => {
                if let Some(player) = self.players.get_mut(&id) {
                    self.chat.add_message(format!("{} left!", player.name));
                }
                if let Some(encryption) = self.encryptions.get_mut(&id) {
                    encryption.index = encryption.name.len() - 1;
                    encryption.action = EncryptionAction::Left;
                }
                self.players.remove(&id);
            }
            LobbyMessage::ReceiveMessage(msg) => {
                self.chat.add_message(msg);
            }
            LobbyMessage::SendMessage(msg) => {
                self.ws_tx
                    .send(ClientMessage::SendMessage(msg).into())
                    .await?;
            }
        }
        Ok(())
    }

    pub fn on_tick(&mut self) {
        let mut encryptions_to_delete = vec![];

        for (id, encryption) in self.encryptions.iter_mut() {
            match encryption.action {
                EncryptionAction::Joined => {
                    if encryption.index < encryption.name.len() {
                        encryption.index += 1;
                    }
                }
                EncryptionAction::Left => {
                    if encryption.name.pop().is_none() {
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
