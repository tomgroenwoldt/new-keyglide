use app::{handle_app_message, AppMessage, AppState};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use warp::Filter;

use crate::routes::{lobbies, play};

mod app;
mod lobby;
mod routes;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Setup app messaging channel, state and message handler.
    let (app_tx, app_rx) = unbounded_channel();
    let app_state = AppState::new(app_tx.clone(), app_rx);
    tokio::spawn(handle_app_message(app_state));

    // Allow warp route handlers to take in the app sending channel as input.
    let app_tx = warp::any().map(move || app_tx.clone());

    let lobbies = warp::path("lobbies")
        .and(warp::ws())
        .and(app_tx.clone())
        .map(|ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>| {
            ws.on_upgrade(|ws| lobbies::handle_connection(ws, app_tx))
        });

    // Allow the play route to accept an optional lobby ID.
    let optional_lobby_id = warp::path::param::<String>()
        .map(Some)
        .or_else(|_| async { Ok::<(Option<String>,), std::convert::Infallible>((None,)) });
    let play = warp::path("play")
        .and(warp::ws())
        .and(app_tx)
        .and(optional_lobby_id)
        .map(
            |ws: warp::ws::Ws, app_tx: UnboundedSender<AppMessage>, lobby_id: Option<String>| {
                ws.on_upgrade(|ws| play::handle_connection(ws, app_tx, lobby_id))
            },
        );

    let routes = play.or(lobbies);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
