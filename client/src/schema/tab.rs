use log::debug;
use strum::{Display, EnumIter};

#[derive(Display, EnumIter)]
pub enum Tab {
    Home,
    Play,
    Logs,
}

impl Tab {
    pub fn next(&self) -> Self {
        let tab = match self {
            Tab::Home => Tab::Play,
            Tab::Play => Tab::Logs,
            Tab::Logs => Tab::Home,
        };
        debug!("Switch from tab {} to next tab {}.", self, tab);
        tab
    }

    pub fn previous(&self) -> Self {
        let tab = match self {
            Tab::Home => Tab::Logs,
            Tab::Play => Tab::Home,
            Tab::Logs => Tab::Play,
        };
        debug!("Switch from tab {} to previous tab {}.", self, tab);
        tab
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Home => 0,
            Tab::Play => 1,
            Tab::Logs => 2,
        }
    }
}
