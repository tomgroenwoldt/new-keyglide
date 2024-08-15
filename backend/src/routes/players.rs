use std::str::FromStr;

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

use crate::{lobby::Player, AppMessage};

pub fn routes(
    app_tx: UnboundedSender<AppMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow warp route handlers to take in the app sending channel as input.
    let app_tx = warp::any().map(move || app_tx.clone());

    let create = warp::path("create")
        .and(warp::ws())
        .and(app_tx.clone())
        .map(|ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
            ws.on_upgrade(|ws| handle_create(ws, app_tx))
        });
    let quickplay = warp::path("quickplay")
        .and(warp::ws())
        .and(app_tx.clone())
        .map(|ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
            ws.on_upgrade(|ws| handle_quickplay(ws, app_tx))
        });
    let join = warp::path!("join" / String)
        .and(warp::ws())
        .and(app_tx)
        .map(
            |lobby_id: String, ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
                ws.on_upgrade(|ws| handle_join(ws, app_tx, lobby_id))
            },
        );
    warp::path("play").and(create.or(join).or(quickplay))
}

pub async fn handle_quickplay(ws: WebSocket, app_tx: UnboundedSender<AppMessage>) {
    let (to_ws, from_ws) = ws.split();

    // Setup player.
    let (player_tx, player_rx) = unbounded_channel();
    let player = Player::new(player_tx);

    // Handle incoming client messages.
    tokio::spawn(receive_and_handle_client_message(
        from_ws,
        app_tx.clone(),
        player.clone(),
    ));

    // Register the new player connection via quickplay.
    if let Err(e) = app_tx.send(AppMessage::AddPlayerViaQuickplay { player }) {
        error!("Error sending via app channel: {e}");
    }

    // Forward messages received through the applicaton channel to the client.
    tokio::spawn(forward_backend_message(to_ws, player_rx));
}

pub async fn handle_join(ws: WebSocket, app_tx: UnboundedSender<AppMessage>, lobby_id: String) {
    let (to_ws, from_ws) = ws.split();

    // Setup player.
    let (player_tx, player_rx) = unbounded_channel();
    let player = Player::new(player_tx);

    // Handle incoming client messages.
    tokio::spawn(receive_and_handle_client_message(
        from_ws,
        app_tx.clone(),
        player.clone(),
    ));

    // Try to add the player to provided lobby.
    let lobby_id = Uuid::from_str(&lobby_id).unwrap();
    if let Err(e) = app_tx.send(AppMessage::AddPlayerToLobby { lobby_id, player }) {
        error!("Error sending via app channel: {e}");
    }

    // Forward messages received through the applicaton channel to the client.
    tokio::spawn(forward_backend_message(to_ws, player_rx));
}

pub async fn handle_create(ws: WebSocket, app_tx: UnboundedSender<AppMessage>) {
    let (to_ws, from_ws) = ws.split();

    // Setup player.
    let (player_tx, player_rx) = unbounded_channel();
    let player = Player::new(player_tx);

    // Handle incoming client messages.
    tokio::spawn(receive_and_handle_client_message(
        from_ws,
        app_tx.clone(),
        player.clone(),
    ));

    if let Err(e) = app_tx.send(AppMessage::CreateLobbyAndAddPlayer { player }) {
        error!("Error sending via app channel: {e}");
    }

    // Forward messages received through the applicaton channel to the client.
    tokio::spawn(forward_backend_message(to_ws, player_rx));
}

async fn receive_and_handle_client_message(
    mut from_ws: SplitStream<WebSocket>,
    app_tx: UnboundedSender<AppMessage>,
    player: Player,
) {
    while let Some(Ok(msg)) = from_ws.next().await {
        if msg.is_close() {
            break;
        }
        let client_message: ClientMessage = serde_json::from_str(msg.to_str().unwrap()).unwrap();
        match client_message {
            ClientMessage::SendMessage { message } => {
                let msg = AppMessage::SendMessage {
                    player: player.clone(),
                    message,
                };
                if let Err(e) = app_tx.send(msg) {
                    error!("Error sending via app channel: {e}");
                }
            }
        };
    }
    // If the player closes his WS connection remove him from the lobby.
    if let Err(e) = app_tx.send(AppMessage::RemovePlayer { player }) {
        error!("Error sending via app channel: {e}");
    }
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
