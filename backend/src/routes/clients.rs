use anyhow::Result;
use common::BackendMessage;
use futures_util::{future::ready, SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::error;
use uuid::Uuid;
use warp::{
    filters::ws::{Message, WebSocket},
    Filter,
};

use crate::app::message::AppMessage;

pub fn routes(
    app_tx: UnboundedSender<AppMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow warp route handlers to take in the app sending channel as input.
    let app_tx = warp::any().map(move || app_tx.clone());

    // Setup client routes.
    warp::path("clients")
        .and(warp::ws())
        .and(app_tx.clone())
        .map(|ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
            ws.on_upgrade(|ws| handle_connection(ws, app_tx))
        })
}

pub async fn handle_connection(ws: WebSocket, app_tx: UnboundedSender<AppMessage>) {
    let (to_ws, mut from_ws) = ws.split();

    // Typecast the websocket sending part to use `BackendMessage directly`.
    let mut to_ws = to_ws.with(|msg: BackendMessage| {
        let res: Result<Message, warp::Error> = Ok(Message::text(
            serde_json::to_string(&msg).expect("Converting message to JSON"),
        ));
        ready(res)
    });

    // Register the new client connection.
    let (client_tx, mut client_rx) = unbounded_channel();
    let client_id = Uuid::new_v4();
    if let Err(e) = app_tx.send(AppMessage::AddClient {
        client_id,
        client_tx,
    }) {
        error!("Error sending via app channel: {e}");
    }

    // Tell the client about all current lobbies.
    if let Err(e) = app_tx.send(AppMessage::CurrentLobbies { client_id }) {
        error!("Error sending via app channel: {e}");
    }

    // If the client closes his WS connection this task will signal the app to
    // remove him from the current clients.
    tokio::spawn(async move {
        while from_ws.next().await.is_some() {}
        if let Err(e) = app_tx.send(AppMessage::RemoveClient { client_id }) {
            error!("Error sending via app channel: {e}");
        }
    });

    // Forward messages received through the applicaton channel to the client
    // WS connection.
    tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if let Err(e) = to_ws.send(msg).await {
                error!("Error sending message via websocket: {e}");
            }
        }
    });
}
