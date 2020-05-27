use git2::Repository;
use sqlx::SqlitePool;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(derive_more::Deref, Clone)]
pub struct Data(Arc<DataInner>);

impl Data {
    pub async fn from_env() -> Result<Self, anyhow::Error> {
        let cfg: Config = envy::from_env()?;
        mkdir_p(&cfg.git_repo)?;

        if let Some(parent) = Path::new(&cfg.db_file).parent() {
            mkdir_p(parent)?;
        }

        let _ = std::fs::create_dir_all(&cfg.git_repo);
        let repo = Repository::discover(&cfg.git_repo)?;

        let url = format!("sqlite://{}", cfg.db_file);
        let pool = SqlitePool::builder()
            .max_size(cfg.db_pool_size)
            .build(&url)
            .await?;

        Ok(Self(Arc::new(DataInner {
            pool,
            repo: Mutex::new(repo),
            repo_path: PathBuf::from(cfg.git_repo),
        })))
    }
}

fn mkdir_p(path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    let path = path.as_ref();
    match std::fs::create_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(anyhow::format_err!(
            "Can't create dir {}: {}",
            path.display(),
            e
        )),
    }
}

pub struct DataInner {
    pub repo: Mutex<git2::Repository>,
    pub pool: sqlx::SqlitePool,
    pub repo_path: PathBuf,
}

#[derive(serde::Deserialize)]
struct Config {
    #[serde(default = "default_repo")]
    git_repo: PathBuf,

    #[serde(default = "default_db_file")]
    db_file: String,

    #[serde(default = "default_db_pool_size")]
    db_pool_size: u32,
}

fn default_repo() -> PathBuf {
    PathBuf::from("/data/repo")
}

fn default_db_file() -> String {
    String::from("/data/db/db.sqlite")
}

fn default_db_pool_size() -> u32 {
    4
}
