use std::{ffi::OsString, os::unix::prelude::*, path::PathBuf};
use warp::Filter;

pub struct WikiArticle {
    pub title: String,
    pub path: PathBuf,
}

pub fn wiki_article(
    data: crate::data::Data,
) -> impl warp::Filter<Extract = (WikiArticle,), Error = warp::Rejection> + Clone {
    warp::path::param().map(move |title: String| {
        let git_repo = data.config.git_repo.as_os_str();
        let mut path = Vec::with_capacity(git_repo.len() + "/".len() + title.len() + ".md".len());
        path.extend(git_repo.as_bytes());
        path.push(b'/');
        path.extend(title.as_bytes());
        path.extend(b".md");
        WikiArticle {
            path: PathBuf::from(OsString::from_vec(path)),
            title,
        }
    })
}
