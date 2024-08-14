use std::time::Duration;

pub static APP_TITLE: &str = "KEYGLIDE";
pub const CHAT_SIZE: usize = 10;
pub const SYMBOLS: &str = "!@#$%^&*()_+-=[]{}|;:,.<>?";

pub static RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
