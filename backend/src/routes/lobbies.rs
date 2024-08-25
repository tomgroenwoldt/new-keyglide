use std::convert::Infallible;

use tokio::sync::{mpsc::UnboundedSender, oneshot};
use warp::Filter;

use common::JoinMode;

use crate::app::message::AppMessage;

pub fn routes(
    app_tx: UnboundedSender<AppMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Allow warp route handlers to take in the app sending channel as input.
    let app_tx = warp::any().map(move || app_tx.clone());

    warp::path!("lobbies" / JoinMode)
        .and(app_tx)
        .and_then(lobby_information)
}

pub async fn lobby_information(
    join_mode: JoinMode,
    app_tx: UnboundedSender<AppMessage>,
) -> Result<impl warp::Reply, Infallible> {
    let (tx, rx) = oneshot::channel();

    let _ = app_tx.send(AppMessage::ProvideLobbyInformation { tx, join_mode });
    let lobby_information = rx.await.expect("Should receive the lobby name.");

    Ok(warp::reply::json(&lobby_information))
}
