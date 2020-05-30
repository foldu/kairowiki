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

        let storage =
            crate::user_storage::SqliteStorage::open(&cfg.db_file, cfg.db_pool_size).await?;

        let _ = std::fs::create_dir_all(&cfg.git_repo);
        let repo = Repository::discover(&cfg.git_repo)?;

        Ok(Self(Arc::new(DataInner {
            user_storage: Box::new(storage),
            repo: Mutex::new(repo),
            repo_path: PathBuf::from(cfg.git_repo),
            port: cfg.port,
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
    pub user_storage: Box<dyn crate::user_storage::UserStorage>,
    pub repo_path: PathBuf,
    pub port: u16,
}

#[derive(serde::Deserialize)]
struct Config {
    #[serde(default = "default_repo")]
    git_repo: PathBuf,

    #[serde(default = "default_db_file")]
    db_file: String,

    #[serde(default = "default_db_pool_size")]
    db_pool_size: u32,

    #[serde(default = "default_port")]
    port: u16,

    #[serde(default = "default_static_dir")]
    static_dir: PathBuf,
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

fn default_port() -> u16 {
    8080
}

fn default_static_dir() -> PathBuf {
    PathBuf::from("/data/static")
}
