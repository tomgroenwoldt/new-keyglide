use std::fmt::Display;

use anyhow::{anyhow, Result};
use crossterm::event::KeyEvent;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;

use client_derive::{CheckChildrenDuplicates, CheckDuplicates};

#[derive(Clone, Debug, Deserialize, CheckChildrenDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct KeyBindings {
    pub movement: Movement,
    pub lobby: Lobby,
    pub join: Join,
    pub popup: Popup,
    pub miscellaneous: Miscellaneous,
}

impl KeyBindings {
    pub fn validate(&self) -> Result<()> {
        if self.children_have_duplicates() {
            // TODO: Change this error when working on https://github.com/tomgroenwoldt/new-keyglide/issues/25.
            return Err(anyhow!("Duplicate key_bindings..."));
        }

        Ok(())
    }
}

impl Display for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut key_binding = format!("<{}", self.code);
        if let Some(modifier) = self.modifiers {
            if modifier.contains(KeyModifiers::SHIFT) {
                key_binding.push_str("+SHIFT");
            }
            if modifier.contains(KeyModifiers::CONTROL) {
                key_binding.push_str("+CTRL");
            }
            if modifier.contains(KeyModifiers::ALT) {
                key_binding.push_str("+ALT");
            }
            if modifier.contains(KeyModifiers::SUPER) {
                key_binding.push_str("+SUPER");
            }
            if modifier.contains(KeyModifiers::HYPER) {
                key_binding.push_str("+HYPER");
            }
            if modifier.contains(KeyModifiers::META) {
                key_binding.push_str("+META");
            }
        }
        key_binding.push('>');
        write!(f, "{}", key_binding)
    }
}

#[derive(Clone, Debug, Deserialize, CheckDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct Movement {
    pub left: KeyBinding,
    pub down: KeyBinding,
    pub right: KeyBinding,
    pub up: KeyBinding,
}

#[derive(Clone, Debug, Deserialize, CheckDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct Miscellaneous {
    pub unfocus: KeyBinding,
    pub toggle_full_screen: KeyBinding,
}

#[derive(Clone, Debug, Deserialize, CheckDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct Lobby {
    pub disconnect: KeyBinding,
    pub focus_chat: KeyBinding,
    pub focus_editor: KeyBinding,
    pub focus_goal: KeyBinding,
    pub toggle_terminal_layout: KeyBinding,
    pub start: KeyBinding,
}

#[derive(Clone, Debug, Deserialize, CheckDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct Join {
    pub focus_lobby_list: KeyBinding,
    pub join_selected: KeyBinding,
    pub quickplay: KeyBinding,
    pub create: KeyBinding,
}

#[derive(Clone, Debug, Deserialize, CheckDuplicates)]
#[serde(rename_all = "kebab-case")]
pub struct Popup {
    pub confirm: KeyBinding,
    pub abort: KeyBinding,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq)]
pub struct KeyBinding {
    #[serde(deserialize_with = "deserialize_user_key")]
    pub code: KeyCode,
    pub modifiers: Option<KeyModifiers>,
}

// Implement our own deserialization for user provided key codes. This
// allows the user to provide simple string values instead of something like
// this for a character, e.g., unfocus.code = { Char = 'q' }.
fn deserialize_user_key<'de, D>(deserializer: D) -> Result<KeyCode, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use serde::de::Error;

    String::deserialize(deserializer)
        .and_then(|string| string_to_key_code(string).map_err(|err| Error::custom(err.to_string())))
}

fn string_to_key_code(key_code: String) -> Result<KeyCode> {
    let code = match key_code.as_str() {
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Tab" => KeyCode::Tab,
        "BackTab" => KeyCode::BackTab,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Null" => KeyCode::Null,
        "Esc" => KeyCode::Esc,
        "CapsLock" => KeyCode::CapsLock,
        "ScrollLock" => KeyCode::ScrollLock,
        "NumLock" => KeyCode::NumLock,
        "PrintScreen" => KeyCode::PrintScreen,
        "Pause" => KeyCode::Pause,
        "Menu" => KeyCode::Menu,
        "KeypadBegin" => KeyCode::KeypadBegin,

        // Only single character keys are allowed.
        c if c.len() == 1 => {
            if let Some(c) = c.chars().next() {
                KeyCode::Char(c)
            } else {
                return Err(anyhow!("Empty key code, even though we checked before."));
            }
        }
        _ => return Err(anyhow!("Invalid key code.")),
    };
    Ok(code)
}

impl PartialEq<KeyBinding> for KeyEvent {
    fn eq(&self, other: &KeyBinding) -> bool {
        if let Some(modifiers) = other.modifiers {
            return self.modifiers == modifiers && self.code == other.code;
        }
        self.code == other.code
    }
}
