use std::collections::BTreeMap;

use anyhow::Result;
use common::{BackendMessage, ClientMessage, Player};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::app::AppMessage;

use super::{chat::Chat, Encryption, EncryptionAction};

pub enum LobbyMessage {
    CloseConnection,
    CurrentPlayers(BTreeMap<Uuid, Player>),
    PlayerJoined(Uuid, Player),
    PlayerLeft(Uuid),
    ReceiveMessage(String),
    SendMessage { message: String },
}

pub struct Lobby {
    pub players: BTreeMap<Uuid, Player>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub chat: Chat,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<LobbyMessage>,
}

impl Lobby {
    pub async fn new(app_tx: UnboundedSender<AppMessage>, lobby_id: Option<Uuid>) -> Result<Self> {
        let mut url = String::from("ws://127.0.0.1:3030/play");
        if let Some(lobby_id) = lobby_id {
            url.push_str(&format!("/{}", lobby_id));
        }
        let (ws_stream, _) = connect_async(url).await?;
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
                    BackendMessage::CloseConnection => {
                        message_tx.send(LobbyMessage::CloseConnection).unwrap();
                    }
                    BackendMessage::SendMessage(msg) => {
                        message_tx.send(LobbyMessage::ReceiveMessage(msg)).unwrap();
                    }
                    BackendMessage::AddPlayer(player_id, player) => {
                        message_tx
                            .send(LobbyMessage::PlayerJoined(player_id, player))
                            .unwrap();
                    }
                    BackendMessage::RemovePlayer(player_id) => {
                        message_tx
                            .send(LobbyMessage::PlayerLeft(player_id))
                            .unwrap();
                    }
                    BackendMessage::LobbyFull => {
                        app_tx.send(AppMessage::LobbyFull).unwrap();
                    }
                    BackendMessage::Unknown => {}
                    BackendMessage::CurrentLobbies(_) => todo!(),
                    BackendMessage::AddLobby(_, _) => todo!(),
                    BackendMessage::RemoveLobby(_) => todo!(),
                }
            }
        });

        Ok(Self {
            players: BTreeMap::new(),
            encryptions: BTreeMap::new(),
            chat: Chat::new(tx),
            ws_tx,
            rx,
        })
    }

    pub async fn handle_message(&mut self, msg: LobbyMessage) -> Result<()> {
        match msg {
            LobbyMessage::CloseConnection => {
                dbg!("closing connection");
                self.ws_tx.close().await?;
            }
            LobbyMessage::CurrentPlayers(players) => {
                for (id, player) in players.iter() {
                    let encryption = Encryption {
                        id: *id,
                        action: EncryptionAction::Joined,
                        index: 0,
                        value: player.name.clone(),
                    };
                    self.encryptions.insert(*id, encryption);
                }
                self.players = players;
            }
            LobbyMessage::PlayerJoined(id, player) => {
                self.chat.add_message(format!("{} joined!", player.name));
                let encryption = Encryption {
                    id,
                    action: EncryptionAction::Joined,
                    index: 0,
                    value: player.name.clone(),
                };
                self.encryptions.insert(id, encryption);
                self.players.insert(id, player);
            }
            LobbyMessage::PlayerLeft(id) => {
                if let Some(player) = self.players.get_mut(&id) {
                    self.chat.add_message(format!("{} left!", player.name));
                }
                if let Some(encryption) = self.encryptions.get_mut(&id) {
                    encryption.index = encryption.value.len() - 1;
                    encryption.action = EncryptionAction::Left;
                }
                self.players.remove(&id);
            }
            LobbyMessage::ReceiveMessage(msg) => {
                self.chat.add_message(msg);
            }
            LobbyMessage::SendMessage { message } => {
                self.ws_tx
                    .send(ClientMessage::SendMessage { message }.into())
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
