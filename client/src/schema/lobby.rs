use std::collections::BTreeMap;

use anyhow::Result;
use common::{BackendMessage, ChallengeFiles, ClientMessage, JoinMode, LobbyInformation, Player};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{debug, error, info};
use ratatui::layout::Size;
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::{
    chat::Chat,
    editor::Editor,
    encryption::{Encryption, EncryptionAction},
};
use crate::{app::AppMessage, schema::goal::Goal};

#[derive(Debug)]
pub enum LobbyMessage {
    CloseConnection,
    EditorTerminated,
    GoalTerminated,
    PlayerJoined(Player),
    PlayerLeft(Uuid),
    ReceiveMessage(String),
    SendMessage { message: String },
    SetLocalPlayerId { id: Uuid },
}

pub struct Lobby {
    pub name: String,
    pub players: BTreeMap<Uuid, Player>,
    pub encryptions: BTreeMap<Uuid, Encryption>,
    pub chat: Chat,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub tx: UnboundedSender<LobbyMessage>,
    pub rx: UnboundedReceiver<LobbyMessage>,
    /// An instance of the users default editor with full interactivity.
    pub editor: Editor,
    /// An instance of the users default editor only capable of resizing.
    pub goal: Goal,
    pub size: Size,
    pub challenge_files: ChallengeFiles,
}

impl Lobby {
    /// # Create new lobby connection
    ///
    /// Connects the player to the backend. Depending on `join_mode` creates or joins a lobby.
    pub async fn new(
        app_tx: UnboundedSender<AppMessage>,
        join_mode: JoinMode,
        size: Size,
    ) -> Result<Self> {
        // First, fetch lobby information of the lobby we want to join.
        let url = format!("http://127.0.0.1:3030/lobbies/{}", join_mode);
        let lobby_information = reqwest::get(url).await?.json::<LobbyInformation>().await?;

        // Connect to lobby with given join mode.
        let url = format!("ws://127.0.0.1:3030/players/{}", lobby_information.id);
        let (ws_stream, _) = connect_async(url).await?;

        // Setup messaging channels.
        let (ws_tx, ws_rx) = ws_stream.split();
        let (tx, rx) = unbounded_channel();

        // Spawn task to handle incoming backend messages.
        let message_tx = tx.clone();
        tokio::spawn(Lobby::handle_backend_message(
            ws_rx,
            message_tx,
            app_tx.clone(),
        ));

        debug!("{:?}", lobby_information);

        let mut encryptions = BTreeMap::new();
        for (id, player) in lobby_information.players.iter() {
            let encryption = Encryption {
                action: EncryptionAction::Joined,
                index: 0,
                value: player.name.clone(),
            };
            encryptions.insert(*id, encryption);
        }

        Ok(Self {
            name: lobby_information.name,
            players: lobby_information.players,
            encryptions,
            chat: Chat::new(tx.clone()),
            ws_tx,
            tx: tx.clone(),
            rx,
            editor: Editor::new(
                size,
                tx.clone(),
                lobby_information.challenge_files.start_file.clone(),
            )?,
            goal: Goal::new(
                size,
                tx,
                lobby_information.challenge_files.goal_file.clone(),
            )?,
            size,
            challenge_files: lobby_information.challenge_files,
        })
    }

    pub async fn handle_message(&mut self, msg: LobbyMessage) -> Result<()> {
        debug!("Handle message {:?}.", msg);

        match msg {
            LobbyMessage::CloseConnection => {
                info!("Close connection to lobby.");
                self.ws_tx.close().await?;
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
            LobbyMessage::SetLocalPlayerId { id } => {
                info!("Received clients player ID {} from the backend.", id);

                if let Some(local_player) = self.encryptions.get_mut(&id) {
                    local_player.value.push_str(" (you)");
                }
            }
            LobbyMessage::EditorTerminated => {
                // Restart the editor if it terminates.
                self.editor = Editor::new(
                    self.size,
                    self.tx.clone(),
                    self.challenge_files.start_file.clone(),
                )?;
            }
            LobbyMessage::GoalTerminated => {
                // Restart the goal editor if it terminates.
                self.goal = Goal::new(
                    self.size,
                    self.tx.clone(),
                    self.challenge_files.goal_file.clone(),
                )?;
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
                BackendMessage::ProvidePlayerId { id } => {
                    message_tx.send(LobbyMessage::SetLocalPlayerId { id })?;
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
        app_tx.send(AppMessage::DisconnectLobby)?;
        Ok(())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.goal.resize(rows, cols)?;
        self.editor.resize(rows, cols)?;
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

    pub fn clean_up(&mut self) -> Result<()> {
        self.goal.terminal.child_killer.kill()?;
        self.editor.terminal.child_killer.kill()?;
        Ok(())
    }
}

// Make sure the terminal instances are killed whenever we drop a lobby.
impl Drop for Lobby {
    fn drop(&mut self) {
        if let Err(e) = self.clean_up() {
            error!("Error cleaning up lobby: {e}");
        }
    }
}
