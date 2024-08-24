use std::{fs, path::Path};

use anyhow::Result;
use log::warn;
use portable_pty::{Child, CommandBuilder};
use ratatui::layout::Size;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::{lobby::LobbyMessage, terminal::Terminal};

pub struct Goal {
    pub terminal: Terminal,
}

impl Goal {
    /// # Create a new goal editor
    ///
    /// Starts a new editor inside a PTY instance that opens up the goal file of
    /// the current lobby.
    pub fn new(
        app_size: Size,
        lobby_tx: UnboundedSender<LobbyMessage>,
        goal_file: Vec<u8>,
    ) -> Result<Self> {
        // Write the start file bytes to a file.
        let file_name = Uuid::new_v4();
        let file_path = format!("/tmp/{}", file_name);
        fs::write(&file_path, goal_file)?;

        // Build the command that opens the goal file fetched from the backend
        // service.
        let mut cmd = CommandBuilder::new("helix");
        let path = Path::new(&file_path);
        cmd.arg(path);

        // Build the terminal and resize it directly.
        let (mut terminal, child) = Terminal::new(app_size, cmd)?;
        terminal.resize(app_size.height, app_size.width)?;

        tokio::spawn(Goal::handle_termination(child, lobby_tx));

        Ok(Self { terminal })
    }

    /// # Handle termination
    ///
    /// Waits for the child process to finish. After finish, message the lobby
    /// and trigger a restart.
    pub async fn handle_termination(
        mut child: Box<dyn Child + Send + Sync>,
        lobby_tx: UnboundedSender<LobbyMessage>,
    ) -> Result<()> {
        child.wait()?;
        warn!("The goal process has completed.");
        lobby_tx.send(LobbyMessage::GoalTerminated)?;
        Ok(())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.terminal.resize(rows, cols)?;
        Ok(())
    }
}
