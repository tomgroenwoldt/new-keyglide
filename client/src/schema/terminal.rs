use std::{
    io::{BufWriter, Write},
    sync::{Arc, Mutex},
};

use anyhow::Result;
use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use log::debug;
use portable_pty::{
    Child, ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem,
};
use ratatui::layout::Size;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tui_term::vt100::Parser;

pub struct Terminal {
    pub sender: UnboundedSender<Bytes>,
    pub master_pty: Box<dyn MasterPty + Send>,
    pub parser: Arc<Mutex<Parser>>,
    pub child_killer: Box<dyn ChildKiller + Send>,
}

impl Terminal {
    pub fn new(
        app_size: Size,
        cmd: CommandBuilder,
    ) -> Result<(Self, Box<dyn Child + Send + Sync>)> {
        let parser = Arc::new(Mutex::new(Parser::new(app_size.height, app_size.width, 0)));
        let pty_system = NativePtySystem::default();

        let size = PtySize::default();
        let pair = pty_system.openpty(size)?;

        // Wait for the child to complete
        let child = pair.slave.spawn_command(cmd)?;

        let mut reader = pair.master.try_clone_reader()?;
        let parser_clone = Arc::clone(&parser);
        tokio::spawn(async move {
            // Consume the output from the child
            // Can't read the full buffer, since that would wait for EOF
            let mut buf = [0u8; 8192];
            let mut processed_buf = Vec::new();
            loop {
                let size = reader
                    .read(&mut buf)
                    .expect("Unable to read from terminal reader.");
                if size == 0 {
                    break;
                }
                if size > 0 {
                    processed_buf.extend_from_slice(&buf[..size]);
                    parser_clone
                        .lock()
                        .expect("Unable to lock terminal parser.")
                        .process(&processed_buf);

                    // Clear the processed portion of the buffer
                    processed_buf.clear();
                }
            }
        });

        let (tx, mut rx) = unbounded_channel::<Bytes>();

        // Drop writer on purpose
        let mut writer = BufWriter::new(pair.master.take_writer()?);
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                writer
                    .write_all(&bytes)
                    .expect("Unable to write bytes to terminal writer.");
                writer.flush().expect("Unable to flush terminal writer.");
            }
        });

        let mut terminal = Self {
            sender: tx,
            master_pty: pair.master,
            parser,
            child_killer: child.clone_killer(),
        };
        terminal.resize(app_size.height, app_size.width)?;

        Ok((terminal, child))
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        let bytes = self.key_to_bytes(event);
        self.sender.send(bytes)?;

        Ok(())
    }

    pub fn key_to_bytes(&self, key: KeyEvent) -> Bytes {
        let bytes = match key.code {
            KeyCode::Char(input) => {
                let mut byte = input as u8;
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    byte = input.to_ascii_uppercase() as u8;
                } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                    byte = input as u8 & 0x1f;
                }
                vec![byte]
            }
            KeyCode::Enter => vec![13],
            KeyCode::Backspace => vec![8],
            KeyCode::Left => vec![27, 91, 68],
            KeyCode::Right => vec![27, 91, 67],
            KeyCode::Up => vec![27, 91, 65],
            KeyCode::Down => vec![27, 91, 66],
            KeyCode::Tab => vec![9],
            KeyCode::Home => vec![27, 91, 72],
            KeyCode::End => vec![27, 91, 70],
            KeyCode::PageUp => vec![27, 91, 53, 126],
            KeyCode::PageDown => vec![27, 91, 54, 126],
            KeyCode::BackTab => vec![27, 91, 90],
            KeyCode::Delete => vec![27, 91, 51, 126],
            KeyCode::Insert => vec![27, 91, 50, 126],
            KeyCode::Esc => vec![27],
            _ => vec![],
        };
        Bytes::from(bytes)
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        debug!("Resize terminal to {} rows and {} columns.", rows, cols);

        let rows = ((rows - 5) as f64 * 0.5) as u16;
        let cols = ((cols - 2) as f64 * 0.8) as u16;
        let pty_size = PtySize {
            rows,
            cols,
            ..Default::default()
        };
        self.master_pty.resize(pty_size)?;
        self.parser
            .lock()
            .expect("Unable to lock terminal parser.")
            .set_size(rows, cols);
        Ok(())
    }
}
