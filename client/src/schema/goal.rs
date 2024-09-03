use std::io::Write;

use anyhow::Result;
use log::warn;
use portable_pty::{Child, CommandBuilder};
use ratatui::layout::{Direction, Size};
use tempfile::NamedTempFile;
use tokio::sync::mpsc::UnboundedSender;

use crate::constants::{GOAL_HEIGHT, TERMINAL_WIDTH};

use super::{lobby::LobbyMessage, terminal::Terminal};

pub struct Goal {
    pub terminal: Terminal,
    pub is_full_screen: bool,
    /// The file the editor is operating on. We keep this inside this struct to
    /// prevent dropping the file (and thereby deleting it).
    #[allow(dead_code)]
    pub file: NamedTempFile,
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
        is_full_screen: bool,
    ) -> Result<Self> {
        // Write the start file bytes to a temporary file.
        let mut file = NamedTempFile::new()?;
        file.write_all(&goal_file)?;

        // Build the command that opens the goal file fetched from the backend
        // service.
        let mut cmd = CommandBuilder::new("helix");
        cmd.arg(file.path());

        // Build the terminal and resize it directly.
        let (terminal, child) = Terminal::new(app_size, cmd)?;

        tokio::spawn(Goal::handle_termination(child, lobby_tx));

        Ok(Self {
            terminal,
            is_full_screen,
            file,
        })
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

    pub fn resize(&mut self, rows: u16, cols: u16, direction: Direction) -> Result<()> {
        if self.is_full_screen {
            self.terminal.resize(rows - 2, cols - 2)?;
            return Ok(());
        }
        let (rows, cols) = match direction {
            Direction::Horizontal => (
                // The full application height - header and borders.
                ((rows - 5) as f64) as u16,
                ((cols - 2) as f64 * TERMINAL_WIDTH * 0.5) as u16 - 1,
            ),
            Direction::Vertical => (
                ((rows - 5) as f64 * GOAL_HEIGHT) as u16 - 1,
                ((cols - 2) as f64 * TERMINAL_WIDTH) as u16,
            ),
        };
        self.terminal.resize(rows, cols)?;
        Ok(())
    }
}
