use std::time::Duration;

/// Empty lobbies survive 30 seconds before being removed.
pub static EMPTY_LOBBY_LIFETIME: Duration = Duration::from_secs(30);
