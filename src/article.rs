use std::{
    ffi::OsStr,
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
    pub fn from_title(title: &ArticleTitle) -> Self {
        Self(Path::new(title.as_ref()).with_extension("md"))
    }
}

#[derive(derive_more::AsRef, derive_more::Deref, derive_more::Display, serde::Serialize)]
pub struct ArticleTitle(String);

impl ArticleTitle {
    pub(crate) fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.extension() != Some(OsStr::new("md")) {
            return Err(Error::InvalidExtension);
        }

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
    pub fn from_title(title: ArticleTitle) -> Self {
        Self {
            path: ArticlePath::from_title(&title),
            title,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Path is not a markdown file")]
    InvalidExtension,

    #[error("Path is not UTF-8")]
    InvalidUTF8,
}

impl warp::reject::Reject for Error {}

pub fn wiki_article(
) -> impl warp::Filter<Extract = (WikiArticle,), Error = std::convert::Infallible> + Clone {
    warp::path::tail().map(move |tail: warp::path::Tail| {
        let title = ArticleTitle::new(urlencoding::decode(tail.as_str()).unwrap());
        WikiArticle::from_title(title)
    })
}
