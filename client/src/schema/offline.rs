use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::{Client, StatusCode};
use tokio::sync::mpsc::UnboundedSender;

use crate::{app::AppMessage, constants::RECONNECT_INTERVAL};

pub struct Offline {
    /// HTTP client to check the service connection.
    pub client: Client,
    pub last_reconnect: Instant,
    pub dot_count: usize,
    pub last_dot: Instant,
    pub app_tx: UnboundedSender<AppMessage>,
}

impl Offline {
    pub fn new(app_tx: UnboundedSender<AppMessage>) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            last_reconnect: Instant::now(),
            dot_count: 0,
            last_dot: Instant::now(),
            app_tx,
        }
    }

    pub async fn try_reconnect(&self) -> Result<()> {
        let Ok(response) = self.client.get("http://127.0.0.1:3030/health").send().await else {
            return Ok(());
        };

        if response.status() == StatusCode::OK {
            self.app_tx.send(AppMessage::ServiceBackOnline)?;
        }
        Ok(())
    }

    pub async fn on_tick(&mut self) -> Result<()> {
        // Try to reconnect every `RECONNECT_INTERVAL`.
        if self.last_reconnect.elapsed() > RECONNECT_INTERVAL {
            self.try_reconnect().await?;
            self.last_reconnect = Instant::now();
        }

        // Add a waiting animation by displaying 0 to 3 dots in a cycle.
        if self.last_dot.elapsed() > Duration::from_millis(500) {
            self.dot_count = (self.dot_count % 3) + 1;
            self.last_dot = Instant::now();
        }
        Ok(())
    }
}
