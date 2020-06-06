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

pub async fn open(path: &str, max_connections: u32) -> Result<sqlx::SqlitePool, ConnectionError> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| CreateParent {
                parent: parent.to_owned(),
            })?;
    }
    let url = format!("sqlite://{}", path);
    sqlx::SqlitePool::builder()
        .max_size(max_connections)
        .build(&url)
        .await
        .with_context(|| Connect {
            path: path.to_owned(),
        })
}
