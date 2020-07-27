use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    #[error("Can't create parent directory for sqlite database in {}: {}", parent.display(), source)]
    CreateParent {
        parent: PathBuf,
        source: std::io::Error,
    },

    #[error("Can't open sqlite database {}: {}", path, source)]
    Connect { path: String, source: sqlx::Error },
}

pub async fn open(path: &str, max_connections: u32) -> Result<sqlx::SqlitePool, ConnectionError> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ConnectionError::CreateParent {
                parent: parent.to_owned(),
                source: e,
            })?;
    }

    sqlx::pool::PoolOptions::new()
        .max_connections(max_connections)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await
        .map_err(|source| ConnectionError::Connect {
            path: path.to_owned(),
            source,
        })
}
