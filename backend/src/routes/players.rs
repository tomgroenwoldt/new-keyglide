use anyhow::Result;
use futures_util::{
    future::ready,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::error;
use uuid::Uuid;
use warp::{
    filters::ws::{Message, WebSocket},
    Filter,
};

use common::{BackendMessage, ClientMessage};

use crate::{player::Player, AppMessage};

pub fn routes(
    app_tx: UnboundedSender<AppMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow warp route handlers to take in the app sending channel as input.
    let app_tx = warp::any().map(move || app_tx.clone());

    warp::path!("players" / Uuid)
        .and(warp::ws())
        .and(app_tx)
        .map(
            |lobby_id: Uuid, ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
                ws.on_upgrade(move |ws| handle_join(ws, app_tx, lobby_id))
            },
        )
}

pub async fn handle_join(ws: WebSocket, app_tx: UnboundedSender<AppMessage>, lobby_id: Uuid) {
    let (to_ws, from_ws) = ws.split();

    // Setup player.
    let (player_tx, player_rx) = unbounded_channel();
    let player = Player::new(player_tx);

    // Handle incoming client messages.
    tokio::spawn(receive_and_handle_client_message(
        from_ws,
        app_tx.clone(),
        player.clone(),
        lobby_id,
    ));

    // Try to add the player to provided lobby.
    let _ = app_tx.send(AppMessage::AddPlayerToLobby { lobby_id, player });

    // Forward messages received through the applicaton channel to the client.
    tokio::spawn(forward_backend_message(to_ws, player_rx));
}

async fn receive_and_handle_client_message(
    mut from_ws: SplitStream<WebSocket>,
    app_tx: UnboundedSender<AppMessage>,
    player: Player,
    lobby_id: Uuid,
) {
    while let Some(Ok(msg)) = from_ws.next().await {
        if msg.is_close() {
            break;
        }
        let client_message: ClientMessage = serde_json::from_str(msg.to_str().unwrap()).unwrap();
        let msg = match client_message {
            ClientMessage::SendMessage { message } => AppMessage::SendMessage {
                player: player.clone(),
                message,
                lobby_id,
            },
            ClientMessage::RequestStart => AppMessage::RequestStart {
                player: player.clone(),
                lobby_id,
            },
            ClientMessage::Progress { progress } => AppMessage::ComputePlayerProgress {
                lobby_id,
                player_id: player.id,
                progress,
            },
        };
        let _ = app_tx.send(msg);
    }
    // If the player closes his WS connection remove him from the lobby.
    let _ = app_tx.send(AppMessage::RemovePlayer { player, lobby_id });
}

async fn forward_backend_message(
    to_ws: SplitSink<WebSocket, Message>,
    mut player_rx: UnboundedReceiver<BackendMessage>,
) {
    // Typecast the websocket sending part to use `BackendMessage directly`.
    let mut to_ws = to_ws.with(|msg: BackendMessage| {
        let res: Result<Message, warp::Error> = Ok(Message::text(
            serde_json::to_string(&msg).expect("Converting message to JSON"),
        ));
        ready(res)
    });

    while let Some(msg) = player_rx.recv().await {
        if let Err(e) = to_ws.send(msg).await {
            error!("Error sending message via websocket: {e}");
        }
    }
}
