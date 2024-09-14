use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use log::{error, warn};
use notify::{
    event::ModifyKind, Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use portable_pty::{Child, CommandBuilder};
use ratatui::layout::{Direction, Size};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use super::terminal::Terminal;
use crate::{
    constants::{EDITOR_HEIGHT, TERMINAL_WIDTH},
    schema::lobby::LobbyMessage,
};

pub struct Editor {
    pub terminal: Terminal,
    pub is_full_screen: bool,
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
        // Get the temporary directory.
        let mut temp_dir = env::temp_dir();
        temp_dir.push("keyglide_challenge");

        // Create the directory.
        if let Err(e) = fs::create_dir_all(&temp_dir) {
            return Err(anyhow!("Failed to create folder: {e}"));
        }
        // Write the start file bytes to file.
        let mut file_path = temp_dir.clone();
        file_path.push(Uuid::new_v4().to_string());

        let mut file = match File::create(&file_path) {
            Ok(file) => file,
            Err(e) => return Err(anyhow!("Error creating file: {e}")),
        };
        if let Err(e) = file.write_all(&start_file) {
            return Err(anyhow!("Error writing to file: {e}"));
        }

        tokio::spawn(watch_progress(
            temp_dir,
            file_path.clone(),
            lobby_tx.clone(),
        ));

        // Build the command that opens the new start file.
        let mut cmd = CommandBuilder::new("helix");
        cmd.arg(&file_path);

        // Build the terminal and resize it directly.
        let (terminal, child) = Terminal::new(app_size, cmd)?;

        // Spawn a task that messages the application after our editor instance
        // terminates and kills the terminal process on app close.
        tokio::spawn(Editor::handle_termination(child, lobby_tx));

        Ok(Self {
            terminal,
            is_full_screen,
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

fn async_watcher() -> notify::Result<(RecommendedWatcher, UnboundedReceiver<notify::Result<Event>>)>
{
    let (tx, rx) = unbounded_channel();
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                if let Err(e) = tx.send(res) {
                    error!("Error sending via async watcher channel: {e}");
                }
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

/// # Watch progress
///
/// Watches the state of the player's start file and on a modifying write event
/// sends the new state via the lobby channel to the backend service.
async fn watch_progress<P: AsRef<Path>>(
    temp_dir: P,
    file_path: PathBuf,
    lobby_tx: UnboundedSender<LobbyMessage>,
) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // We have to watch recursively inside a folder because of
    // the way editors handle file writes.
    // See https://docs.rs/notify/latest/notify/#editor-behaviour.
    watcher.watch(temp_dir.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) if event.paths.contains(&file_path) => {
                if let EventKind::Modify(ModifyKind::Data(_)) = event.kind {
                    let progress = fs::read(&file_path).unwrap();
                    if let Err(e) = lobby_tx.send(LobbyMessage::SendProgress { progress }) {
                        error!("Error sending player progress via lobby channel: {e}");
                    }
                }
            }
            Err(e) => error!("watch error: {:?}", e),
            _ => {}
        }
    }

    Ok(())
}
