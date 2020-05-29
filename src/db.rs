use crate::password::PasswordHash;
use sqlx::SqliteConnection;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Database error: {}", err))]
    Sql { err: sqlx::Error },

    #[snafu(display("User already exists"))]
    UserExists,

    #[snafu(display("Email already used"))]
    EmailExists,
}

impl warp::reject::Reject for Error {}

impl From<sqlx::Error> for Error {
    fn from(other: sqlx::Error) -> Error {
        match other {
            sqlx::Error::Database(ref db_err) => match db_err.message() {
                // FIXME: quality
                "UNIQUE constraint failed: wiki_user.name" => Error::UserExists,
                "UNIQUE constraint failed: wiki_user.email" => Error::EmailExists,
                _ => Error::Sql { err: other },
            },
            _ => Error::Sql { err: other },
        }
    }
}

pub async fn create_user(db: &mut SqliteConnection, usr: NewUser) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO wiki_user(name, email, pass_hash) VALUES (?, ?, ?)",
        &usr.name,
        &usr.email,
        usr.pass_hash.as_ref()
    )
    .execute(db)
    .await?;

    Ok(())
}
