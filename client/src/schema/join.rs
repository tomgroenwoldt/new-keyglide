use std::collections::BTreeMap;

use anyhow::Result;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
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

use super::{
    connection::Connection,
    encryption::{Encryption, EncryptionAction},
    lobby::Lobby,
};
use crate::app::{App, AppMessage};

pub struct Join {
    pub lobbies: BTreeMap<Uuid, common::Lobby>,
    pub total_clients: usize,
    pub total_players: usize,
    pub selected_lobby: Option<Uuid>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub rx: UnboundedReceiver<JoinMessage>,
}

pub enum JoinMessage {
    CurrentLobbies(BTreeMap<Uuid, common::Lobby>),
    CloseConnection,
    AddLobby(Uuid, common::Lobby),
    RemoveLobby(Uuid),
    ConnectionCounts { players: usize, clients: usize },
}

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
        tokio::spawn(Join::handle_backend_message(ws_rx, message_tx, app_tx));

        Ok(Self {
            lobbies: BTreeMap::new(),
            total_clients: 0,
            total_players: 0,
            selected_lobby: None,
            encryptions: BTreeMap::new(),
            ws_tx,
            rx,
        })
    }

    pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
        let Connection::Join(ref mut join) = app.connection else {
            return Ok(());
        };
        match key.code {
            // Join a selected lobby.
            KeyCode::Enter => {
                if let Some(selected_lobby) = join.selected_lobby {
                    join.ws_tx.close().await?;
                    let join_mode = JoinMode::Join(selected_lobby);
                    let lobby = Lobby::new(app.message_tx.clone(), join_mode).await?;
                    app.connection = Connection::Lobby(lobby);
                    app.focused_component = None;
                }
            }
            KeyCode::Char(c) => match c {
                'j' => {
                    if let Some(lobby_id) = join.selected_lobby {
                        join.selected_lobby = join
                            .lobbies
                            .range(lobby_id..)
                            .nth(1)
                            .or_else(|| join.lobbies.range(..=lobby_id).next())
                            .map(|(id, _)| *id);
                    } else {
                        join.selected_lobby = join
                            .lobbies
                            .first_key_value()
                            .map(|(lobby_id, _)| *lobby_id);
                    }
                }
                'k' => {
                    if let Some(lobby_id) = join.selected_lobby {
                        join.selected_lobby = join
                            .lobbies
                            .range(..lobby_id)
                            .next_back()
                            .or_else(|| join.lobbies.iter().next_back())
                            .map(|(lobby_id, _)| *lobby_id);
                    } else {
                        join.selected_lobby =
                            join.lobbies.last_key_value().map(|(lobby_id, _)| *lobby_id);
                    }
                }
                // Join lobby via quickplay.
                'q' => {
                    join.ws_tx.close().await?;
                    let lobby = Lobby::new(app.message_tx.clone(), JoinMode::Quickplay).await?;
                    app.connection = Connection::Lobby(lobby);
                    app.focused_component = None;
                }
                // Create and join a new lobby.
                'n' => {
                    join.ws_tx.close().await?;
                    let lobby = Lobby::new(app.message_tx.clone(), JoinMode::Create).await?;
                    app.connection = Connection::Lobby(lobby);
                    app.focused_component = None;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub async fn handle_message(&mut self, msg: JoinMessage) -> Result<()> {
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
                self.ws_tx.close().await?;
            }
            JoinMessage::AddLobby(lobby_id, lobby) => {
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
                self.lobbies.remove(&lobby_id);
            }
            JoinMessage::ConnectionCounts { players, clients } => {
                self.total_clients = clients;
                self.total_players = players;
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
            if msg.is_close() {
                return Ok(());
            }
            let backend_message: BackendMessage = msg.into();
            match backend_message {
                BackendMessage::CloseConnection => {
                    let _ = message_tx.send(JoinMessage::CloseConnection);
                    return Ok(());
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
                BackendMessage::ConnectionCounts { clients, players } => {
                    let _ = message_tx.send(JoinMessage::ConnectionCounts { clients, players });
                }
                _ => {}
            }
        }

        app_tx.send(AppMessage::ServiceDisconnected)?;
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
