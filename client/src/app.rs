use anyhow::Result;
use futures_util::SinkExt;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
    schema::{editor::Editor, lobby::Lobby},
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

    pub lobby: Option<Lobby>,
    pub focused_component: Option<FocusedComponent>,

    pub exit: bool,
}

pub enum FocusedComponent {
    Chat,
    Editor,
    ExitPopup,
}

pub enum AppMessage {
    EditorTerminated,
}

impl App {
    pub fn new(area: Rect) -> Self {
        let (message_tx, message_rx) = unbounded_channel();
        App {
            current_tab: Tab::Home,
            editor: None,
            area,
            message_tx,
            message_rx,
            lobby: None,
            focused_component: None,
            exit: false,
        }
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
            self.handle_key_for_focused_component(key)?;
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

    fn handle_key_for_focused_component(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(ref mut focused_component) = self.focused_component {
            match focused_component {
                FocusedComponent::Chat => {
                    if let Some(ref mut lobby) = self.lobby {
                        lobby.chat.handle_key_event(key)?;
                    }
                }
                FocusedComponent::Editor => {
                    if let Some(ref mut editor) = self.editor {
                        editor.handle_key_event(key)?;
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
                // Disconnect from existing lobby.
                if c == 'd' && self.lobby.is_some() {
                    if let Some(ref mut lobby) = self.lobby {
                        lobby.ws_tx.close().await?;
                    }
                    self.lobby = None;
                }
                // Connect to the lobby if there is none.
                if c == 'j' && self.lobby.is_none() {
                    let lobby = Lobby::new().await?;
                    self.lobby = Some(lobby);
                }
                // Focus the chat (only possible when connected to a lobby).
                if c == 'i' && self.lobby.is_some() {
                    self.focused_component = Some(FocusedComponent::Chat);
                }
                // Focus the editor (only possible when connected to a lobby).
                if c == 's' && self.lobby.is_some() {
                    self.focused_component = Some(FocusedComponent::Editor);

                    // If there is no editor running, start one.
                    if self.editor.is_none() {
                        let new_editor = Editor::new(self.area, self.message_tx.clone())?;
                        self.editor = Some(new_editor);
                    }
                }
            }
        };
        Ok(())
    }

    pub fn handle_message(&mut self, msg: AppMessage) -> Result<()> {
        match msg {
            AppMessage::EditorTerminated => {
                self.editor = None;
            }
        }
        Ok(())
    }

    pub fn on_tick(&mut self) -> Result<()> {
        if let Some(ref mut lobby) = self.lobby {
            lobby.on_tick();
        }
        Ok(())
    }
}
