use uuid::Uuid;

pub mod chat;
pub mod editor;
pub mod join;
pub mod lobby;

pub struct Encryption {
    pub id: Uuid,
    pub action: EncryptionAction,
    pub index: usize,
    pub value: String,
}

pub enum EncryptionAction {
    Joined,
    Left,
}
