use std::time::Duration;

/// Empty lobbies survive 30 seconds before being removed.
pub static EMPTY_LOBBY_LIFETIME: Duration = Duration::from_secs(30);
/// Lobbies start ten seconds after a start request.
pub static LOBBY_START_TIMER: Duration = Duration::from_secs(10);
/// Lobbies are up to two minutes in progress.
pub static MAX_LOBBY_PLAY_TIME: Duration = Duration::from_secs(60 * 2);
/// After one player finished, the lobby play time is reduced.
pub static REDUCED_LOBBY_PLAY_TIME: Duration = Duration::from_secs(10);
/// Lobbies are ten seconds in the finish state.
pub static LOBBY_FINISH_TIME: Duration = Duration::from_secs(10);
