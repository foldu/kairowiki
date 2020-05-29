use snafu::ResultExt;
use std::path::PathBuf;

#[derive(snafu::Snafu, Debug)]
pub enum ConnectionError {
    #[snafu(display("Can't create parent directory for sqlite database in {}: {}", parent.display(), source))]
    CreateParent {
        parent: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Can't open sqlite database {}: {}", path, source))]
    Connect { path: String, source: sqlx::Error },
}

impl From<sqlx::Error> for super::Error {
    fn from(other: sqlx::Error) -> super::Error {
        match other {
            sqlx::Error::Database(ref db_err) => match db_err.message() {
                // FIXME: quality
                "UNIQUE constraint failed: wiki_user.name" => super::Error::UserExists,
                "UNIQUE constraint failed: wiki_user.email" => super::Error::EmailExists,
                _ => super::Error::Generic(other.into()),
            },
            _ => super::Error::Generic(other.into()),
        }
    }
}

pub struct SqliteStorage(sqlx::SqlitePool);

impl SqliteStorage {
    pub async fn open(path: &str, max_connections: u32) -> Result<Self, ConnectionError> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| CreateParent {
                    parent: parent.to_owned(),
                })?;
        }

        let url = format!("sqlite://{}", path);
        let pool = sqlx::SqlitePool::builder()
            .max_size(max_connections)
            .build(&url)
            .await
            .with_context(|| Connect {
                path: path.to_owned(),
            })?;

        Ok(Self(pool))
    }
}

#[async_trait::async_trait]
impl super::UserStorage for SqliteStorage {
    async fn check_credentials(&self, name: &str, pass: &str) -> Result<(), super::Error> {
        let mut cxn = self.0.acquire().await?;

        let hash = match sqlx::query!("SELECT pass_hash FROM wiki_user WHERE name = ?", name)
            .fetch_optional(&mut cxn)
            .await
            .map(|row| row.map(|row| row.pass_hash))
        {
            Ok(Some(hash)) => {
                Ok(PasswordHash::from_vec(hash).expect("Invalid password in database"))
            }
            Ok(None) => Err(super::Error::UserDoesNotExist),
            Err(e) => Err(e.into()),
        }?;

        if hash.verify(pass) {
            Ok(())
        } else {
            Err(super::Error::InvalidPassword)
        }
    }

    async fn register(&self, info: &crate::forms::Register) -> Result<(), super::Error> {
        let cxn = self.0.acquire().await?;
        let hash = PasswordHash::from_password(&info.password);

        sqlx::query!(
            "INSERT INTO wiki_user(name, email, pass_hash) VALUES (?, ?, ?)",
            &info.name,
            &info.email,
            hash.as_ref()
        )
        .execute(cxn)
        .await?;

        Ok(())
    }
}

#[derive(derive_more::AsRef)]
pub struct PasswordHash(Vec<u8>);

#[derive(Debug, thiserror::Error)]
#[error("Blob is not a valid password hash")]
pub struct InvalidPasswordHash;

impl PasswordHash {
    const SALT_LEN: usize = 8;
    const HASH_LEN: usize = 32;
    const LEN: usize = Self::SALT_LEN + Self::HASH_LEN;

    pub fn from_password(pass: &str) -> Self {
        use rand::Rng;

        let mut ret = vec![0_u8; Self::LEN];
        rand::thread_rng().fill(&mut ret[0..Self::SALT_LEN]);

        let hash = argon2::hash_raw(
            pass.as_bytes(),
            &ret[0..Self::SALT_LEN],
            &argon2::Config::default(),
        )
        .unwrap();

        ret[Self::SALT_LEN..].copy_from_slice(&hash);

        Self(ret)
    }

    pub fn from_vec(v: Vec<u8>) -> Result<Self, InvalidPasswordHash> {
        if v.len() == Self::LEN {
            Ok(Self(v))
        } else {
            Err(InvalidPasswordHash)
        }
    }

    pub fn verify(&self, pass: &str) -> bool {
        assert!(self.0.len() == Self::LEN);
        argon2::verify_raw(
            pass.as_bytes(),
            &self.0[0..Self::SALT_LEN],
            &self.0[Self::SALT_LEN..],
            &argon2::Config::default(),
        )
        .unwrap()
    }
}
