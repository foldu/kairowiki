pub mod sqlite;
pub use sqlite::SqliteStorage;

#[async_trait::async_trait]
pub trait UserStorage: Sync + Send {
    async fn register(&self, info: &crate::forms::Register) -> Result<(), Error>;

    async fn check_credentials(&self, name: &str, pass: &str) -> Result<UserId, Error>;
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq)]
pub struct UserId(i32);

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

    #[error("{0}")]
    Generic(Box<dyn std::error::Error + Send + Sync>),
}

impl warp::reject::Reject for Error {}
