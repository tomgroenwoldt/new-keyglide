use std::str::FromStr;

use anyhow::Result;
use fake::{faker::name::raw::Name, locales::EN, Fake};
use futures_util::{future::ready, SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::error;
use uuid::Uuid;
use warp::filters::ws::{Message, WebSocket};

use common::{BackendMessage, ClientMessage, Player};

use crate::AppMessage;

pub async fn handle_connection(
    ws: WebSocket,
    app_tx: UnboundedSender<AppMessage>,
    lobby_id: Option<String>,
) {
    let (to_ws, mut from_ws) = ws.split();

    // Typecast the websocket sending part to use `BackendMessage directly`.
    let mut to_ws = to_ws.with(|msg: BackendMessage| {
        let res: Result<Message, warp::Error> = Ok(Message::text(
            serde_json::to_string(&msg).expect("Converting message to JSON"),
        ));
        ready(res)
    });

    // Register the new player connection. If the player provided a lobby ID
    // try to join it. Otherwise, join via quickplay.
    let player_id = Uuid::new_v4();
    let (player_tx, mut player_rx) = unbounded_channel();
    let player = Player {
        tx: player_tx,
        name: Name(EN).fake(),
    };
    if let Some(lobby_id) = lobby_id {
        let lobby_id = Uuid::from_str(&lobby_id).unwrap();
        let _ = app_tx.send(AppMessage::AddPlayerToLobby {
            lobby_id,
            player_id,
            player,
        });
    } else {
        let _ = app_tx.send(AppMessage::AddPlayerViaQuickplay { player_id, player });
    }

    tokio::spawn(async move {
        while let Some(Ok(msg)) = from_ws.next().await {
            if msg.is_close() {
                break;
            }
            let client_message: ClientMessage =
                serde_json::from_str(msg.to_str().unwrap()).unwrap();
            match client_message {
                ClientMessage::SendMessage { message } => {
                    let msg = AppMessage::SendMessage { player_id, message };
                    app_tx.send(msg).unwrap();
                }
            };
        }
        // If the player closes his WS connection remove him from the lobby.
        let _ = app_tx.send(AppMessage::RemovePlayer { player_id });
    });

    // Forward messages received through the applicaton channel to the client
    // WS connection.
    tokio::spawn(async move {
        while let Some(msg) = player_rx.recv().await {
            if let Err(e) = to_ws.send(msg).await {
                error!("Error sending message via websocket: {e}");
            }
        }
    });
}
