use crate::migrations::{Migration, MigrationInfo, NeedsMigration};

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
    pub fn new(pool: sqlx::SqlitePool) -> NeedsMigration<Self> {
        NeedsMigration::new(Self(pool))
    }
}

impl MigrationInfo for SqliteStorage {
    fn migrations(&self) -> &'static [Migration] {
        &[migration!("user_schema")]
    }
}

#[async_trait::async_trait]
impl super::UserStorage for SqliteStorage {
    fn registration_supported(&self) -> bool {
        true
    }

    async fn check_credentials(
        &self,
        name: &str,
        pass: &str,
    ) -> Result<super::UserId, super::Error> {
        let mut cxn = self.0.acquire().await?;

        let (id, hash) =
            match sqlx::query!("SELECT id, pass_hash FROM wiki_user WHERE name = ?", name)
                .fetch_optional(&mut cxn)
                .await
            {
                Ok(Some(row)) => Ok((
                    row.id,
                    PasswordHash::from_vec(row.pass_hash).expect("Invalid password in database"),
                )),
                Ok(None) => Err(super::Error::UserDoesNotExist),
                Err(e) => Err(e.into()),
            }?;

        if hash.verify(pass) {
            Ok(super::UserId(id))
        } else {
            Err(super::Error::InvalidPassword)
        }
    }

    async fn register(&self, info: &crate::forms::Register) -> Result<(), super::Error> {
        let mut cxn = self.0.acquire().await?;
        let hash = PasswordHash::from_password(&info.password);

        sqlx::query!(
            "INSERT INTO wiki_user(name, email, pass_hash) VALUES (?, ?, ?)",
            &info.name,
            &info.email,
            hash.as_ref()
        )
        .execute(&mut *cxn)
        .await?;

        Ok(())
    }

    async fn fetch_account(&self, id: super::UserId) -> Result<super::UserAccount, super::Error> {
        let mut cxn = self.0.acquire().await?;
        sqlx::query!("SELECT name, email FROM wiki_user WHERE id = ?", id.0)
            .fetch_optional(&mut cxn)
            .await?
            .map(|row| super::UserAccount {
                name: row.name,
                email: row.email,
            })
            .ok_or(super::Error::UserDoesNotExist)
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
