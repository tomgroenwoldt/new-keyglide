use std::{collections::BTreeMap, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::Display;
#[cfg(feature = "client")]
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub mod constants;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Progress { progress: Vec<u8> },
    SendMessage { message: String },
    RequestStart,
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
            Message::Close(_) => Self::CloseConnection,
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                Self::Unknown
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub progress: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyListItem {
    pub name: String,
    pub player_count: usize,
    pub status: LobbyStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Display, PartialEq, Eq)]
pub enum LobbyStatus {
    #[strum(to_string = "Waiting for players")]
    WaitingForPlayers,
    /// Holds the time the game starts.
    #[strum(to_string = "About to start")]
    AboutToStart(DateTime<Utc>),
    /// Holds the time the game finishes.
    #[strum(to_string = "In progress")]
    InProgress(DateTime<Utc>),
    /// Holds the time the game reset.
    #[strum(to_string = "Just finished")]
    Finish(DateTime<Utc>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyInformation {
    pub id: Uuid,
    pub name: String,
    pub status: LobbyStatus,
    pub owner: Option<Uuid>,
    pub players: BTreeMap<Uuid, Player>,
    pub challenge_files: ChallengeFiles,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeFiles {
    /// File all players start from.
    pub start_file: Vec<u8>,
    /// The goal state of the start file.
    pub goal_file: Vec<u8>,
}

#[derive(Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum JoinMode {
    /// Client wants to join a non-full lobby or create a new one.
    Quickplay,
    /// Clients wants to join a specific lobby.
    #[strum(to_string = "{lobby_id}")]
    Join { lobby_id: Uuid },
    /// Client wants to create a new lobby.
    Create,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseJoinModeError;

impl FromStr for JoinMode {
    type Err = ParseJoinModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create" => Ok(JoinMode::Create),
            "quickplay" => Ok(JoinMode::Quickplay),
            s => {
                if let Ok(lobby_id) = Uuid::from_str(s) {
                    Ok(JoinMode::Join { lobby_id })
                } else {
                    Err(ParseJoinModeError)
                }
            }
        }
    }
}

#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Clone, Debug, Serialize)]
pub enum BackendMessage {
    CurrentLobbies(BTreeMap<Uuid, LobbyListItem>),
    AddLobby(Uuid, LobbyListItem),
    UpdateLobbyPlayerCount { id: Uuid, player_count: usize },
    UpdateLobbyStatus { id: Uuid, status: LobbyStatus },
    RemoveLobby(Uuid),
    LobbyFull,
    LobbyNotWaitingForPlayers,
    ConnectionCounts { clients: usize, players: usize },

    SendLobbyInformation(LobbyInformation),
    ProvidePlayerId { id: Uuid },
    AssignOwner { id: Uuid },
    AddPlayer(Player),
    RemovePlayer(Uuid),
    StatusUpdate { status: LobbyStatus },
    UpdatePlayerProgress { player_id: Uuid, progress: f64 },

    SendMessage(String),
    CloseConnection,
    Unknown,
}
