use app::{
    message::{handle_app_message, AppMessage},
    App,
};
use tokio::sync::mpsc::unbounded_channel;
use warp::{reply, Filter};

use crate::routes::{clients, players};

mod app;
mod constants;
mod lobby;
mod routes;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Setup app, communication channel and message handler.
    let (app_tx, app_rx) = unbounded_channel();
    let app = App::new(app_tx.clone(), app_rx);
    tokio::spawn(handle_app_message(app));

    let health = warp::path("health").map(reply);

    // Build routes.
    let play_routes = players::routes(app_tx.clone());
    let client_routes = clients::routes(app_tx.clone());

    // Serve routes.
    let routes = health.or(client_routes.or(play_routes));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
