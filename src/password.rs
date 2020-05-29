#[derive(derive_more::AsRef)]
pub struct PasswordHash(Vec<u8>);

impl PasswordHash {
    pub fn from_password(s: &str) -> Self {
        // FIXME:
        Self(s.as_bytes().to_owned())
    }

    pub fn from_raw(hash: Vec<u8>) -> Option<Self> {
        Some(Self(hash))
    }
}
