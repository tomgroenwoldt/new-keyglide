use strum::{Display, EnumIter};

#[derive(Display, EnumIter)]
pub enum Tab {
    Home,
    Play,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Home => Tab::Play,
            Tab::Play => Tab::Home,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Home => Tab::Play,
            Tab::Play => Tab::Home,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Home => 0,
            Tab::Play => 1,
        }
    }
}
