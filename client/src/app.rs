use anyhow::Result;
use futures_util::SinkExt;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
    schema::{editor::Editor, join::Join, lobby::Lobby},
    tab::Tab,
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
}

impl Connection {
    async fn new() -> Result<Self> {
        let join = Join::new().await?;
        Ok(Connection::Join(join))
    }
}

pub enum FocusedComponent {
    Chat,
    Editor,
    ExitPopup,
    Lobbies,
}

pub enum AppMessage {
    LobbyFull,
    EditorTerminated,
}

impl App {
    pub async fn new(area: Rect) -> Result<Self> {
        let (message_tx, message_rx) = unbounded_channel();
        let connection = Connection::new().await?;
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
        if self.focused_component.is_some() {
            self.handle_key_for_focused_component(key).await?;
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

    async fn handle_key_for_focused_component(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(ref mut focused_component) = self.focused_component {
            match focused_component {
                FocusedComponent::Chat => {
                    if let Connection::Lobby(ref mut lobby) = self.connection {
                        lobby.chat.handle_key_event(key)?;
                    }
                }
                FocusedComponent::Editor => {
                    if let Some(ref mut editor) = self.editor {
                        editor.handle_key_event(key)?;
                    }
                }
                FocusedComponent::Lobbies => {
                    if let Connection::Join(ref mut join) = self.connection {
                        match key.code {
                            KeyCode::Enter => {
                                join.ws_tx.close().await?;
                                let lobby =
                                    Lobby::new(self.message_tx.clone(), join.selected_lobby)
                                        .await?;
                                self.connection = Connection::Lobby(lobby);
                                self.focused_component = None;
                            }
                            KeyCode::Char(c) => match c {
                                'j' => {
                                    if let Some(lobby_id) = join.selected_lobby {
                                        join.selected_lobby = join
                                            .lobbies
                                            .range(lobby_id..)
                                            .nth(1)
                                            .or_else(|| join.lobbies.range(..=lobby_id).next())
                                            .map(|(id, _)| *id);
                                    } else {
                                        join.selected_lobby = join
                                            .lobbies
                                            .first_key_value()
                                            .map(|(lobby_id, _)| *lobby_id);
                                    }
                                }
                                'k' => {
                                    if let Some(lobby_id) = join.selected_lobby {
                                        join.selected_lobby = join
                                            .lobbies
                                            .range(..lobby_id)
                                            .next_back()
                                            .or_else(|| join.lobbies.iter().next_back())
                                            .map(|(lobby_id, _)| *lobby_id);
                                    } else {
                                        join.selected_lobby = join
                                            .lobbies
                                            .last_key_value()
                                            .map(|(lobby_id, _)| *lobby_id);
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                FocusedComponent::ExitPopup => {
                    if let KeyCode::Char(c) = key.code {
                        match c {
                            'y' => {
                                self.exit = true;
                            }
                            'n' => {
                                self.focused_component = None;
                            }
                            _ => {}
                        }
                    }
                }
            };
        }
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
                        // Connect to the lobby if there is none.
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
            AppMessage::LobbyFull => {
                if let Connection::Lobby(ref mut lobby) = self.connection {
                    lobby.ws_tx.close().await?;
                    let join = Join::new().await?;
                    self.connection = Connection::Join(join);
                }
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
        }
        Ok(())
    }
}
