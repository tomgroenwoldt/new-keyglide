use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::SinkExt;
use ratatui::{
    backend::Backend,
    crossterm::{
        self,
        event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    },
    layout::Rect,
    Terminal,
};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::sleep,
};

#[cfg(feature = "audio")]
use crate::audio::play_audio;
use crate::{
    constants::RECONNECT_DURATION,
    schema::{
        editor::Editor, focused_component::FocusedComponent, join::Join, lobby::Lobby, tab::Tab,
    },
    ui,
};

pub struct App {
    /// The currently selected tab.
    pub current_tab: Tab,
    /// An instance of the users default editor.
    pub editor: Option<Editor>,
    /// The current size of the terminal the application is running in.
    pub area: Rect,
    pub message_tx: UnboundedSender<AppMessage>,
    pub message_rx: UnboundedReceiver<AppMessage>,

    pub connection: Connection,
    pub focused_component: Option<FocusedComponent>,

    pub exit: bool,
}

pub enum Connection {
    Join(Join),
    Lobby(Lobby),
    Offline,
}

impl Connection {
    /// # Create a new connection
    ///
    /// Tries to connect the client to the backend. If this fails it returns the
    /// `Connection::Offline` variant and spawns a task that tries to reconnect
    /// continously.
    /// Notifies the application on a successful reconnect.
    async fn new(app_tx: UnboundedSender<AppMessage>) -> Result<Self> {
        let connection = match Join::new().await {
            Ok(join) => Connection::Join(join),
            Err(_) => {
                tokio::spawn(async move {
                    while reqwest::get("http://127.0.0.1:3030/health").await.is_err() {
                        sleep(RECONNECT_DURATION).await;
                    }
                    app_tx
                        .send(AppMessage::Reconnected)
                        .expect("Unable to send via app channel.");
                });
                Connection::Offline
            }
        };
        Ok(connection)
    }
}

pub enum AppMessage {
    EditorTerminated,
    DisconnectLobby,
    Reconnected,
}

impl App {
    pub async fn new(area: Rect) -> Result<Self> {
        let (message_tx, message_rx) = unbounded_channel();
        let connection = Connection::new(message_tx.clone()).await?;
        let app = App {
            current_tab: Tab::Home,
            editor: None,
            area,
            message_tx,
            message_rx,
            connection,
            focused_component: None,
            exit: false,
        };
        Ok(app)
    }

    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> Result<()> {
        let mut last_tick = Instant::now();
        while !self.exit {
            terminal.draw(|f| ui::draw(f, self))?;

            // Handle terminal events.
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                let event = event::read()?;

                match event {
                    // If the editor is running and focused it takes precedence over
                    // anything else.
                    Event::Key(key) => {
                        self.on_key(key).await?;
                    }
                    Event::Resize(cols, rows) => {
                        if let Some(ref mut editor) = self.editor {
                            editor.resize(rows, cols)?;
                        }
                        self.area = terminal.size()?;
                    }
                    _ => {}
                }
            }

            // Handle app messages sent from other tasks.
            if let Ok(msg) = self.message_rx.try_recv() {
                self.handle_message(msg).await?;
            }

            // Handle lobby messages.
            match self.connection {
                Connection::Lobby(ref mut lobby) => {
                    if let Ok(msg) = lobby.rx.try_recv() {
                        lobby.handle_message(msg).await?;
                    }
                }
                Connection::Join(ref mut join) => {
                    if let Ok(msg) = join.rx.try_recv() {
                        join.handle_message(msg).await?;
                    }
                }
                Connection::Offline => {}
            }

            // Handle application ticks. This is mainly used for handling
            // animations.
            if last_tick.elapsed() >= tick_rate {
                self.on_tick()?;
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    /// # Move to the next tab
    ///
    /// Selects the next tab.
    pub fn on_right(&mut self) {
        self.current_tab = self.current_tab.next();
    }

    /// # Move to previous tab
    ///
    /// Selects the previous tab.
    pub fn on_left(&mut self) {
        self.current_tab = self.current_tab.previous();
    }

    pub async fn on_key(&mut self, key: KeyEvent) -> Result<()> {
        // CTRL + Q is the universal combination to unfocus components or quit
        // the application if no component is focused.
        if let KeyCode::Char('q') = key.code {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if self.focused_component.is_some() {
                    self.focused_component = None;
                } else {
                    self.focused_component = Some(FocusedComponent::ExitPopup);
                }
                return Ok(());
            }
        }

        // Check whether there is a component focused. Such components receive
        // direct user input and take precedence.
        if let Some(focused_component) = self.focused_component.clone() {
            focused_component.handle_key(self, key).await?;
        } else if let KeyCode::Char(c) = key.code {
            // First, handle general purpose key bindings.
            match c {
                'h' => self.on_left(),
                'l' => self.on_right(),
                // Then, handle key bindings per tab.
                c => {
                    self.handle_key_per_tab(c).await?;
                }
            };
        };
        Ok(())
    }

    async fn handle_key_per_tab(&mut self, c: char) -> Result<()> {
        match self.current_tab {
            Tab::Home => {}
            Tab::Play => {
                match self.connection {
                    Connection::Join(_) => {
                        if c == 'i' {
                            self.focused_component = Some(FocusedComponent::Lobbies);
                        }
                    }
                    Connection::Lobby(ref mut lobby) => {
                        // Disconnect from existing lobby.
                        if c == 'd' {
                            lobby.ws_tx.close().await?;
                            let join = Join::new().await?;
                            self.connection = Connection::Join(join);
                        }
                        // Focus the chat.
                        if c == 'i' {
                            self.focused_component = Some(FocusedComponent::Chat);
                        }
                        // Focus the editor.
                        if c == 's' {
                            self.focused_component = Some(FocusedComponent::Editor);

                            // If there is no editor running, start one.
                            if self.editor.is_none() {
                                let new_editor = Editor::new(self.area, self.message_tx.clone())?;
                                self.editor = Some(new_editor);
                            }
                        }
                    }
                    Connection::Offline => {}
                }
            }
        };
        Ok(())
    }

    pub async fn handle_message(&mut self, msg: AppMessage) -> Result<()> {
        match msg {
            AppMessage::EditorTerminated => {
                self.editor = None;
            }
            AppMessage::DisconnectLobby => {
                if let Connection::Lobby(ref mut lobby) = self.connection {
                    lobby.ws_tx.close().await?;
                    let join = Join::new().await?;
                    self.connection = Connection::Join(join);
                }
            }
            AppMessage::Reconnected => {
                self.connection = Connection::new(self.message_tx.clone()).await?;

                #[cfg(feature = "audio")]
                play_audio("assets/back_online.mp3")?;
            }
        }
        Ok(())
    }

    pub fn on_tick(&mut self) -> Result<()> {
        match self.connection {
            Connection::Join(ref mut join) => {
                join.on_tick();
            }
            Connection::Lobby(ref mut lobby) => {
                lobby.on_tick();
            }
            Connection::Offline => {}
        }
        Ok(())
    }
}
