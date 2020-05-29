pub mod sqlite;
pub use sqlite::SqliteStorage;

#[async_trait::async_trait]
pub trait UserStorage: Sync + Send {
    async fn register(&self, info: &NewUser) -> Result<(), Error>;

    async fn check_credentials(&self, name: &str, pass: &str) -> Result<(), Error>;
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

    #[error("{0}")]
    Generic(Box<dyn std::error::Error + Send + Sync>),
}

impl warp::reject::Reject for Error {}

pub struct NewUser {
    pub name: String,
    pub email: String,
    pub pass_hash: PasswordHash,
}

#[derive(derive_more::AsRef)]
pub struct PasswordHash(Vec<u8>);

#[derive(Debug, thiserror::Error)]
#[error("Blob is not a valid password hash")]
pub struct InvalidPasswordHash;

impl PasswordHash {
    const SALT_LEN: usize = 8;
    // FIXME: don't know the actual length
    const HASH_LEN: usize = 16;

    pub fn from_password(pass: &str) -> Self {
        use rand::Rng;
        let mut ret = vec![0_u8; Self::SALT_LEN + Self::HASH_LEN];
        rand::thread_rng().fill(&mut ret[0..Self::SALT_LEN]);
        let hash = argon2::hash_raw(
            pass.as_bytes(),
            &ret[0..Self::SALT_LEN],
            &argon2::Config::default(),
        )
        .unwrap();
        println!("{}", hash.len());
        ret[Self::SALT_LEN..].copy_from_slice(&hash);
        Self(ret)
    }

    pub fn from_vec(v: Vec<u8>) -> Result<Self, InvalidPasswordHash> {
        // FIXME: check bounds
        Ok(Self(v))
    }

    pub fn verify(&self, pass: &str) -> bool {
        argon2::verify_raw(
            pass.as_bytes(),
            &self.0[0..Self::SALT_LEN],
            &self.0[Self::SALT_LEN..],
            &argon2::Config::default(),
        )
        .unwrap()
    }
}
