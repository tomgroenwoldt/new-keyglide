use std::io::Write;

use anyhow::{anyhow, Result};
use log::warn;
use portable_pty::{Child, CommandBuilder};
use ratatui::layout::{Direction, Size};
use tempfile::NamedTempFile;
use tokio::sync::mpsc::UnboundedSender;

use super::terminal::Terminal;
use crate::{
    constants::{EDITOR_HEIGHT, TERMINAL_WIDTH},
    schema::lobby::LobbyMessage,
};

pub struct Editor {
    pub terminal: Terminal,
    pub is_full_screen: bool,
    /// The file the editor is operating on. We keep this inside this struct to
    /// prevent dropping the file (and thereby deleting it).
    #[allow(dead_code)]
    pub file: NamedTempFile,
}

impl Editor {
    /// # Create a new editor
    ///
    /// Starts a new editor inside a PTY instance that opens up the start file
    /// of the current lobby.
    pub fn new(
        app_size: Size,
        lobby_tx: UnboundedSender<LobbyMessage>,
        start_file: Vec<u8>,
        is_full_screen: bool,
    ) -> Result<Self> {
        // Write the start file bytes to a temporary file.
        let mut file = match NamedTempFile::new() {
            Ok(file) => file,
            Err(e) => return Err(anyhow!("Error creating temporary file: {e}")),
        };
        if let Err(e) = file.write_all(&start_file) {
            return Err(anyhow!("Error writing to temporary file: {e}"));
        }

        // Build the command that opens the new start file.
        let mut cmd = CommandBuilder::new("helix");
        cmd.arg(file.path());

        // Build the terminal and resize it directly.
        let (terminal, child) = Terminal::new(app_size, cmd)?;

        // Spawn a task that messages the application after our editor instance
        // terminates and kills the terminal process on app close.
        tokio::spawn(Editor::handle_termination(child, lobby_tx));

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
        warn!("The editor process terminated.");
        lobby_tx.send(LobbyMessage::EditorTerminated)?;
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
                ((rows - 5) as f64 * EDITOR_HEIGHT) as u16 - 1,
                ((cols - 2) as f64 * TERMINAL_WIDTH) as u16,
            ),
        };
        self.terminal.resize(rows, cols)?;
        Ok(())
    }
}
