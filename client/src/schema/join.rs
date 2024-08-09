use std::collections::BTreeMap;

use anyhow::Result;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use common::{constants::MAX_LOBBY_SIZE, BackendMessage, Lobby};

use super::{Encryption, EncryptionAction};

pub struct Join {
    pub lobbies: BTreeMap<Uuid, Lobby>,
    pub selected_lobby: Option<Uuid>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<JoinMessage>,
}

pub enum JoinMessage {
    CurrentLobbies(BTreeMap<Uuid, Lobby>),
    CloseConnection,
    AddLobby(Uuid, Lobby),
    RemoveLobby(Uuid),
}

impl Join {
    pub async fn new() -> Result<Self> {
        let (ws_stream, _) = connect_async("ws://127.0.0.1:3030/lobbies").await?;
        let (ws_tx, mut ws_rx) = ws_stream.split();

        let (tx, rx) = unbounded_channel();
        let message_tx = tx.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                let backend_message: BackendMessage = msg.into();
                match backend_message {
                    BackendMessage::CloseConnection => {
                        let _ = message_tx.send(JoinMessage::CloseConnection);
                    }
                    BackendMessage::CurrentLobbies(lobbies) => {
                        let _ = message_tx.send(JoinMessage::CurrentLobbies(lobbies));
                    }
                    BackendMessage::AddLobby(lobby_id, lobby) => {
                        let _ = message_tx.send(JoinMessage::AddLobby(lobby_id, lobby));
                    }
                    BackendMessage::RemoveLobby(lobby_id) => {
                        let _ = message_tx.send(JoinMessage::RemoveLobby(lobby_id));
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            lobbies: BTreeMap::new(),
            selected_lobby: None,
            encryptions: BTreeMap::new(),
            ws_tx,
            rx,
        })
    }

    pub async fn handle_message(&mut self, msg: JoinMessage) -> Result<()> {
        match msg {
            JoinMessage::CurrentLobbies(lobbies) => {
                for (id, lobby) in lobbies.iter() {
                    let value =
                        format!("{} ({}/{})", lobby.name, lobby.player_count, MAX_LOBBY_SIZE);
                    let encryption = Encryption {
                        id: *id,
                        action: EncryptionAction::Joined,
                        index: 0,
                        value,
                    };
                    self.encryptions.insert(*id, encryption);
                }
                self.lobbies = lobbies;
            }
            JoinMessage::CloseConnection => {
                self.ws_tx.close().await?;
            }
            JoinMessage::AddLobby(lobby_id, lobby) => {
                let value = format!("{} ({}/{})", lobby.name, lobby.player_count, MAX_LOBBY_SIZE);
                let encryption = Encryption {
                    id: lobby_id,
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
                self.lobbies.remove(&lobby_id);
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
