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
