use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    SendMessage(String),
}

impl From<ClientMessage> for Message {
    fn from(value: ClientMessage) -> Self {
        let text = serde_json::to_string(&value).expect("Converting message to JSON");
        Message::text(text)
    }
}

impl From<Message> for BackendMessage {
    fn from(value: Message) -> Self {
        match value {
            Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
            Message::Close(_) => BackendMessage::CloseConnection,
            _ => BackendMessage::Unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackendMessage {
    PlayerJoined(Uuid, Player),
    CurrentPlayers(BTreeMap<Uuid, Player>),
    PlayerLeft(Uuid),
    SendMessage(String),
    CloseConnection,
    Unknown,
}
