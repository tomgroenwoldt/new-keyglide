use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "backend")]
use tokio::sync::mpsc::UnboundedSender;
#[cfg(not(feature = "backend"))]
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub mod constants;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    SendMessage { message: String },
}

#[cfg(not(feature = "backend"))]
impl From<ClientMessage> for Message {
    fn from(value: ClientMessage) -> Self {
        let text = serde_json::to_string(&value).expect("Converting message to JSON");
        Message::text(text)
    }
}

#[cfg(not(feature = "backend"))]
impl From<Message> for BackendMessage {
    fn from(value: Message) -> Self {
        match value {
            Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
            Message::Close(_) => BackendMessage::CloseConnection,
            _ => BackendMessage::Unknown,
        }
    }
}

#[cfg_attr(not(feature = "backend"), derive(Deserialize))]
#[derive(Clone, Debug, Serialize)]
pub struct Player {
    pub name: String,
    #[cfg(feature = "backend")]
    #[serde(skip)]
    pub tx: UnboundedSender<BackendMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lobby {
    pub name: String,
    pub player_count: usize,
}

#[cfg_attr(not(feature = "backend"), derive(Deserialize))]
#[derive(Clone, Debug, Serialize)]
pub enum BackendMessage {
    CurrentLobbies(BTreeMap<Uuid, Lobby>),
    AddLobby(Uuid, Lobby),
    RemoveLobby(Uuid),
    LobbyFull,

    CurrentPlayers(BTreeMap<Uuid, Player>),
    AddPlayer(Uuid, Player),
    RemovePlayer(Uuid),

    SendMessage(String),
    CloseConnection,
    Unknown,
}
