use anyhow::Context;
use git2::Repository;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(derive_more::Deref, Clone)]
pub struct Data(Arc<DataInner>);

impl Data {
    pub async fn from_env() -> Result<Self, anyhow::Error> {
        let cfg: Config = envy::from_env()?;
        mkdir_p(&cfg.git_repo)?;
        match Repository::open(&cfg.git_repo) {
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                Repository::init(&cfg.git_repo).context("Could not init git repo")?;
            }
            other => other.map(|_| ()).context("Could not open git repository")?,
        };

        let pool = crate::sqlite::open(&cfg.db_file, cfg.db_pool_size).await?;
        let migrations = crate::migrations::Migrations::new(pool.clone()).await?;
        let user_storage = migrations
            .run(crate::user_storage::SqliteStorage::new(pool.clone()).await?)
            .await?;

        Ok(Self(Arc::new(DataInner {
            user_storage: Box::new(user_storage),
            config: cfg,
        })))
    }
}

impl Data {
    pub fn wiki(&self) -> Wiki {
        Wiki {
            name: &self.config.wiki_name,
            footer: &self.config.footer,
            logo: "/static/dancing_green_fluorescent_alien.gif",
        }
    }

    pub fn registration_possible(&self) -> bool {
        self.user_storage.registration_supported() && self.config.registration_enabled
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
    pub user_storage: Box<dyn crate::user_storage::UserStorage>,
    pub config: Config,
}

pub struct Wiki<'a> {
    pub name: &'a str,
    pub footer: &'a str,
    pub logo: &'a str,
}

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_repo")]
    pub git_repo: PathBuf,

    #[serde(default = "default_db_file")]
    pub db_file: String,

    #[serde(default = "default_db_pool_size")]
    pub db_pool_size: u32,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_static_dir")]
    pub static_dir: PathBuf,

    #[serde(default = "default_wiki_name")]
    pub wiki_name: String,

    #[serde(default = "default_footer")]
    pub footer: String,

    #[serde(default = "default_home_wiki_page")]
    pub home_wiki_page: String,

    #[serde(default = "tru")]
    pub registration_enabled: bool,
}

fn tru() -> bool {
    true
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
    PathBuf::from("/static")
}

fn default_wiki_name() -> String {
    "kairowiki".to_owned()
}

fn default_footer() -> String {
    "kairowiki".into()
}

fn default_home_wiki_page() -> String {
    "kairowiki".to_string()
}
