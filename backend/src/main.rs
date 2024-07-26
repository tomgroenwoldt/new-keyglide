use std::collections::BTreeMap;

use anyhow::Result;
use common::{BackendMessage, ClientMessage, Player};
use futures_util::{future::ready, SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, info};
use urlencoding::decode;
use uuid::Uuid;
use warp::{
    filters::ws::{Message, WebSocket},
    Filter,
};

pub struct Lobby {
    pub rx: UnboundedReceiver<LobbyMessage>,
    pub players: BTreeMap<Uuid, Player>,
    pub player_txs: BTreeMap<Uuid, UnboundedSender<BackendMessage>>,
}

impl Lobby {
    fn broadcast(&mut self, msg: BackendMessage) -> Result<()> {
        for tx in self.player_txs.values_mut() {
            tx.send(msg.clone())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum LobbyMessage {
    Init(Uuid),
    RegisterPlayer(Uuid, String, UnboundedSender<BackendMessage>),
    SendMessage(String, String),
    RemovePlayer(Uuid),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (tx, rx) = unbounded_channel();
    let tx = warp::any().map(move || tx.clone());
    tokio::spawn(start_lobby(rx));

    let routes = warp::any()
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .and(tx)
        .and(warp::path::param())
        .map(
            |ws: warp::ws::Ws, tx: UnboundedSender<LobbyMessage>, username: String| {
                // And then our closure will be called when it completes...
                ws.on_upgrade(|ws| handle_connection(ws, tx, username))
            },
        );

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn start_lobby(rx: UnboundedReceiver<LobbyMessage>) -> Result<()> {
    let mut lobby = Lobby {
        rx,
        players: BTreeMap::new(),
        player_txs: BTreeMap::new(),
    };

    while let Some(msg) = lobby.rx.recv().await {
        match msg {
            LobbyMessage::Init(player_id) => {
                let message = BackendMessage::CurrentPlayers(lobby.players.clone());
                let player_tx = lobby.player_txs.get(&player_id).unwrap();
                player_tx.send(message)?;
            }
            LobbyMessage::RegisterPlayer(id, name, tx) => {
                let player = Player { name };
                lobby.players.insert(id, player.clone());
                lobby.player_txs.insert(id, tx);
                info!("{} joined the lobby.", player.name);
                let message = BackendMessage::PlayerJoined(id, player);
                lobby.broadcast(message)?;
            }
            LobbyMessage::RemovePlayer(id) => {
                if let Some(player) = lobby.players.get(&id) {
                    info!("{} left the lobby.", player.name);
                }
                lobby.player_txs.remove(&id);
                lobby.players.remove(&id);
                let message = BackendMessage::PlayerLeft(id);
                lobby.broadcast(message)?;
            }
            LobbyMessage::SendMessage(player, msg) => {
                let message = BackendMessage::SendMessage(format!("{player}: {msg}"));
                lobby.broadcast(message)?;
            }
        }
    }

    Ok(())
}

async fn handle_connection(
    ws: WebSocket,
    lobby_tx: UnboundedSender<LobbyMessage>,
    username: String,
) {
    let (to_ws, mut from_ws) = ws.split();
    let mut to_ws = to_ws.with(|msg: BackendMessage| {
        let res: Result<Message, warp::Error> = Ok(Message::text(
            serde_json::to_string(&msg).expect("Converting message to JSON"),
        ));
        ready(res)
    });
    let id = Uuid::new_v4();

    // Register the new player connection.
    let (tx, mut rx) = unbounded_channel();
    let username = decode(&username).unwrap().to_string();
    lobby_tx
        .send(LobbyMessage::RegisterPlayer(id, username.clone(), tx))
        .unwrap();

    // Tell the new player about all current players.
    lobby_tx.send(LobbyMessage::Init(id)).unwrap();

    tokio::spawn(async move {
        while let Some(msg) = from_ws.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Error receiving message via websocket: {e}");
                    break;
                }
            };
            if msg.is_close() {
                // lobby_tx.send(LobbyMessage::RemovePlayer(id)).unwrap();
                break;
            }
            let client_message: ClientMessage =
                serde_json::from_str(msg.to_str().unwrap()).unwrap();
            let lobby_message = match client_message {
                ClientMessage::SendMessage(msg) => LobbyMessage::SendMessage(username.clone(), msg),
            };
            lobby_tx.send(lobby_message).unwrap();
        }
        lobby_tx.send(LobbyMessage::RemovePlayer(id)).unwrap();
    });

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = to_ws.send(msg).await {
                error!("Error sending message via websocket: {e}");
            }
        }
    });
}
