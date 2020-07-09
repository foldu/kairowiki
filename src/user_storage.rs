pub mod sqlite;
pub use sqlite::SqliteStorage;

#[async_trait::async_trait]
pub trait UserStorage: Sync + Send {
    fn registration_supported(&self) -> bool {
        false
    }

    async fn register(&self, _info: &crate::forms::Register) -> Result<(), Error> {
        Err(Error::RegistrationUnsupported)
    }

    async fn check_credentials(&self, name: &str, pass: &str) -> Result<UserAccount, Error>;
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq)]
pub struct UserId(i32);

#[derive(Clone)]
pub struct UserAccount {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("User does not exist")]
    UserDoesNotExist,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("User already exists")]
    UserExists,

    #[error("Email is already registered")]
    EmailExists,

    #[error("Backend does not support registration")]
    RegistrationUnsupported,

    #[error("{0}")]
    Generic(Box<dyn std::error::Error + Send + Sync>),
}

impl warp::reject::Reject for Error {}

