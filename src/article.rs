use std::{
    ffi::{OsStr, OsString},
    os::unix::prelude::*,
    path::{Path, PathBuf},
};
use warp::Filter;

pub struct WikiArticle {
    pub title: ArticleTitle,
    pub path: ArticlePath,
}

#[derive(derive_more::AsRef, derive_more::Deref)]
pub struct ArticlePath(PathBuf);

impl ArticlePath {
    pub fn from_title(root: impl AsRef<Path>, title: &ArticleTitle) -> Self {
        let root = root.as_ref().as_os_str();

        let mut path = Vec::with_capacity(root.len() + "/".len() + title.len() + ".md".len());
        path.extend(root.as_bytes());
        path.push(b'/');
        path.extend(title.as_bytes());
        path.extend(b".md");

        Self(PathBuf::from(OsString::from_vec(path)))
    }
}

#[derive(derive_more::AsRef, derive_more::Deref, derive_more::Display, serde::Serialize)]
pub struct ArticleTitle(String);

impl ArticleTitle {
    pub fn from_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let root = root.as_ref();
        if path.extension() != Some(OsStr::new("md")) {
            return Err(Error::InvalidExtension);
        }

        let path = path.strip_prefix(root).unwrap();

        path.with_extension("")
            .into_os_string()
            .into_string()
            .map_err(|_| Error::InvalidUTF8)
            .map(Self)
    }

    pub fn new(s: String) -> Self {
        Self(s)
    }
}

impl WikiArticle {
    pub fn from_title(root: impl AsRef<Path>, title: ArticleTitle) -> Self {
        Self {
            path: ArticlePath::from_title(root, &title),
            title,
        }
    }

    pub async fn read_to_string(&self) -> Result<String, Error> {
        match tokio::fs::read_to_string(&self.path.as_ref()).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::DoesNotExist),
            other => other.map_err(Error::Io),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Article does not exist")]
    DoesNotExist,

    #[error("Can't read article: {0}")]
    Io(std::io::Error),

    #[error("Path is not a markdown file")]
    InvalidExtension,

    #[error("Path is not UTF-8")]
    InvalidUTF8,
}

impl warp::reject::Reject for Error {}

pub fn wiki_article(
    ctx: crate::context::Context,
) -> impl warp::Filter<Extract = (WikiArticle,), Error = std::convert::Infallible> + Clone {
    warp::path::tail().map(move |tail: warp::path::Tail| {
        let title = ArticleTitle::new(urlencoding::decode(tail.as_str()).unwrap());
        WikiArticle::from_title(&ctx.config.git_repo, title)
    })
}

