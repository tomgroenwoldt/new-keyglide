use fake::{faker::name::raw::Name, locales::EN, Fake};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use common::BackendMessage;

#[derive(Clone, Debug)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub tx: UnboundedSender<BackendMessage>,
    pub progress: f64,
}

impl Player {
    pub fn new(tx: UnboundedSender<BackendMessage>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Name(EN).fake(),
            tx,
            progress: 0.0,
        }
    }

    pub fn to_common_player(&self) -> common::Player {
        common::Player {
            id: self.id,
            name: self.name.clone(),
            progress: self.progress,
        }
    }
}
