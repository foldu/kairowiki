use crate::{
    file_storage::{self, FileStorage},
    git::Repo,
    index::Index,
    markdown::MarkdownRenderer,
    serde::SeparatedList,
    user_storage::{self, UserAccount},
};
use anyhow::Context as AnyhowContext;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(derive_more::Deref, Clone)]
pub struct Context(Arc<DataInner>);

impl Context {
    pub async fn from_env() -> Result<Self, anyhow::Error> {
        let cfg: Config = envy::from_env()?;
        mkdir_p(&cfg.git_repo)?;
        let repo = Repo::open_or_init(cfg.git_repo.clone(), &cfg.home_wiki_page)?;

        let pool = crate::sqlite::open(&cfg.db_file, cfg.db_pool_size).await?;
        let migrations = crate::migrations::Migrations::new(pool.clone()).await?;
        let user_storage = migrations
            .run(user_storage::SqliteStorage::new(pool.clone()))
            .await?;

        let file_storage = FileStorage::new(
            pool.clone(),
            file_storage::Config {
                storage_path: cfg.storage_path.clone(),
                // TODO: make this configurable
                allowed_mime_types: &cfg.allowed_mime_types.0,
                route: "/storage".to_owned(),
                mime_types_path: &cfg.mime_types_path,
            },
        )?;

        let file_storage = migrations.run(file_storage).await?;

        let theme_path = cfg.static_dir.join("hl.css");

        let repo_read = repo.read()?;
        let index = Index::open(&cfg.index_dir, &repo_read)
            .await
            .context("Can't set up search index")?;

        Ok(Self(Arc::new(DataInner {
            repo,
            index,
            user_storage: Box::new(user_storage),
            file_storage,
            markdown_renderer: MarkdownRenderer::new(&cfg.syntax_theme_name, theme_path)?,
            config: cfg,
        })))
    }
}

impl Context {
    pub fn wiki<'a>(&'a self, account: &'a Option<UserAccount>) -> Wiki {
        Wiki {
            login_status: account,
            name: &self.config.wiki_name,
            footer: &self.config.footer,
            logo: "/static/logo.svg",
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
    pub file_storage: crate::file_storage::FileStorage,
    pub markdown_renderer: MarkdownRenderer,
    pub repo: Repo,
    pub index: Index,
}

pub struct Wiki<'a> {
    pub name: &'a str,
    pub footer: &'a str,
    pub logo: &'a str,
    pub login_status: &'a Option<UserAccount>,
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

    #[serde(default = "default_storage_path")]
    pub storage_path: String,

    #[serde(default = "default_theme_name")]
    pub syntax_theme_name: String,

    pub domain: Option<url::Url>,

    #[serde(default = "default_ip_addr")]
    pub ip_addr: IpAddr,

    #[serde(default = "default_mime_types_path")]
    pub mime_types_path: PathBuf,

    #[serde(default = "default_mime_types")]
    pub allowed_mime_types: crate::serde::SeparatedList<mime::Mime>,

    #[serde(default)]
    pub dangerously_allow_script_eval_for_development_only: bool,

    #[serde(default = "default_index_dir")]
    pub index_dir: PathBuf,
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
    PathBuf::from("/usr/lib/kairowiki/static")
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

fn default_storage_path() -> String {
    "/data/storage".into()
}

fn default_theme_name() -> String {
    "InspiredGitHub".to_owned()
}

fn default_ip_addr() -> IpAddr {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
}

pub fn default_mime_types_path() -> PathBuf {
    PathBuf::from("/etc/mime.types")
}

fn default_mime_types() -> SeparatedList<mime::Mime> {
    SeparatedList(vec![
        mime::IMAGE_JPEG,
        mime::IMAGE_PNG,
        mime::IMAGE_GIF,
        mime::IMAGE_SVG,
        "image/webp".parse().unwrap(),
    ])
}

fn default_index_dir() -> PathBuf {
    PathBuf::from("/data/index")
}

