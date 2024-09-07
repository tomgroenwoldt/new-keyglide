use std::time::Duration;

pub static APP_TITLE: &str = "KEYGLIDE";
/// The height of the editor instance displaying the actual editor (the user is
/// editing in) in percent of the whole application size.
pub static EDITOR_HEIGHT: f64 = 0.5;
/// The height of the editor instance displaying the goal in percent of the
/// whole application size.
pub static GOAL_HEIGHT: f64 = 0.5;
/// Width of the sidebar in the play tab in percent of the whole application
/// size.
pub static PLAY_SIDE_WIDTH: f64 = 0.2;

pub static RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
pub static SYMBOLS: &str = "!@#$%^&*()_+-=[]{}|;:,.<>?";
/// Width of the terminals in percent of the whole application size.
pub static TERMINAL_WIDTH: f64 = 0.8;
