pub mod read;
pub mod write;

pub use read::HistoryEntry;

use crate::article::ArticlePath;
use git2::Repository;
use std::{fs, io::Write, os::unix::prelude::*, path::PathBuf};
use tokio::sync::Mutex;

// FIXME: better error messages
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Could not open or init repository in path {}: {}", path.display(), err)]
    RepoOpen { path: PathBuf, err: git2::Error },

    #[error("Can't create post-receive-hook: {}", _0)]
    HookCreate(std::io::Error),
}

impl warp::reject::Reject for Error {}

pub struct Repo {
    path: PathBuf,
    repo: Mutex<Repository>,
}

impl Repo {
    pub fn open_or_init(path: PathBuf, home_page: &str) -> Result<Self, Error> {
        let repo_path = path.clone();
        let repo = match Repository::open_bare(&repo_path) {
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                let repo = Repository::init_bare(&repo_path)?;
                let article = crate::article::WikiArticle::from_title(
                    crate::article::ArticleTitle::new(home_page.to_owned()),
                );

                write::write_and_commit_file(
                    &repo,
                    None,
                    &write::CommitInfo {
                        path: &article.path,
                        signature: git2::Signature::now("system", "system").unwrap(),
                        msg: "Initial commit",
                    },
                    "This is the home page of your new wiki. Click on edit to put something here.",
                )?;

                let post_receive_hook =
                    askama::Template::render(&crate::templates::PostReceiveHook::new()).unwrap();

                let hook_path = path.join("hooks/post-receive");
                let ret: Result<(), std::io::Error> = try {
                    let mut file = fs::OpenOptions::new()
                        .mode(0o755)
                        .write(true)
                        .create(true)
                        .open(hook_path)?;
                    file.write(post_receive_hook.as_bytes())?;
                    file.flush()?;
                };

                ret.map_err(Error::HookCreate)?;

                Ok(repo)
            }
            other => other,
        };

        match repo {
            Ok(repo) => Ok(Self {
                path,
                repo: Mutex::new(repo),
            }),
            Err(e) => Err(Error::RepoOpen { path, err: e }),
        }
    }

    pub fn read(&self) -> Result<read::ReadOnly, Error> {
        let repo = Repository::open_bare(&self.path)?;
        Ok(read::ReadOnly { repo })
    }

    pub async fn write(&self) -> write::RepoLock<'_> {
        write::RepoLock {
            repo: self.repo.lock().await,
        }
    }
}

fn repo_head(repo: &Repository) -> Result<Option<git2::Reference<'_>>, git2::Error> {
    match repo.head() {
        Ok(head) => Ok(Some(head)),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(e) => Err(e),
    }
}

fn get_tree_path<'a>(
    tree: &'a git2::Tree,
    path: &ArticlePath,
) -> Result<Option<git2::TreeEntry<'a>>, git2::Error> {
    match tree.get_path(path.as_ref()) {
        Ok(ent) => Ok(Some(ent)),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn get_blob_oid<'a>(
    tree: &'a git2::Tree,
    tree_path: &ArticlePath,
) -> Result<Option<git2::Oid>, Error> {
    match get_tree_path(&tree, tree_path)? {
        Some(ent) if ent.kind() == Some(git2::ObjectType::Blob) => Ok(Some(ent.id())),
        _ => Ok(None),
    }
}

fn get_as_blob<'a>(
    repo: &'a Repository,
    tree: &git2::Tree,
    path: &ArticlePath,
) -> Result<Option<git2::Blob<'a>>, git2::Error> {
    let ent = match get_tree_path(tree, path)? {
        Some(ent) => ent,
        None => return Ok(None),
    };

    let obj = ent.to_object(repo)?;
    match obj.into_blob() {
        Ok(blob) => Ok(Some(blob)),
        Err(_) => Ok(None),
    }
}
