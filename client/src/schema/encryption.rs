#[derive(Debug)]
pub struct Encryption {
    pub action: EncryptionAction,
    pub index: usize,
    pub value: String,
}

#[derive(Debug)]
pub enum EncryptionAction {
    Joined,
    Left,
}

impl Encryption {
    pub fn new(value: String) -> Self {
        Self {
            action: EncryptionAction::Joined,
            index: 0,
            value,
        }
    }
}
