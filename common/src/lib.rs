use std::collections::BTreeMap;

#[cfg(feature = "backend")]
use fake::{faker::name::raw::Name, locales::EN, Fake};
use serde::{Deserialize, Serialize};
#[cfg(feature = "backend")]
use tokio::sync::mpsc::UnboundedSender;
#[cfg(feature = "client")]
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub mod constants;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    SendMessage { message: String },
}

#[cfg(feature = "client")]
impl From<ClientMessage> for Message {
    fn from(value: ClientMessage) -> Self {
        let text = serde_json::to_string(&value).expect("Converting message to JSON");
        Message::text(text)
    }
}

#[cfg(feature = "client")]
impl From<Message> for BackendMessage {
    fn from(value: Message) -> Self {
        match value {
            Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
            Message::Close(_) => BackendMessage::CloseConnection,
            _ => BackendMessage::Unknown,
        }
    }
}

#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Clone, Debug, Serialize)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    #[cfg(feature = "backend")]
    #[serde(skip)]
    pub tx: UnboundedSender<BackendMessage>,
}

#[cfg(feature = "backend")]
impl Player {
    pub fn new(tx: UnboundedSender<BackendMessage>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Name(EN).fake(),
            tx,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lobby {
    pub name: String,
    pub player_count: usize,
}

#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Clone, Debug, Serialize)]
pub enum BackendMessage {
    CurrentLobbies(BTreeMap<Uuid, Lobby>),
    AddLobby(Uuid, Lobby),
    RemoveLobby(Uuid),
    LobbyFull,
    ConnectionCounts { clients: usize, players: usize },

    ProvideLobbyName { name: String },
    ProvidePlayerId { id: Uuid },
    CurrentPlayers(BTreeMap<Uuid, Player>),
    AddPlayer(Player),
    RemovePlayer(Uuid),

    SendMessage(String),
    CloseConnection,
    Unknown,
}
