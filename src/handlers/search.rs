use crate::{article::ArticleTitle, data::Data, templates, user_storage::UserAccount};
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
};
use tokio::stream::StreamExt;

// would be nicer if this was a stream so I could just filter/map normally
// but that's too annoying to implement
async fn search<TryTransform, Ret>(root: PathBuf, mut try_transform: TryTransform) -> Vec<Ret>
where
    TryTransform: FnMut(&PathBuf, &Metadata) -> Option<Ret>,
{
    let mut ret = Vec::new();
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        if let Ok(meta) = tokio::fs::metadata(&path).await {
            if let Some(elt) = try_transform(&path, &meta) {
                ret.push(elt);
            }
            if meta.file_type().is_dir() {
                append_entries(&path, &mut stack).await;
            }
        }
    }

    ret
}

async fn append_entries(path: impl AsRef<Path>, vec: &mut Vec<PathBuf>) {
    if let Ok(mut read_dir) = tokio::fs::read_dir(path).await {
        while let Some(ent) = read_dir.next().await {
            if let Ok(ent) = ent {
                vec.push(ent.path());
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    query: String,
}

pub async fn search_repo(
    data: Data,
    account: Option<UserAccount>,
    query: SearchQuery,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    // unwrap ok because query is escaped
    let re = regex::Regex::new(&format!("(?i){}", regex::escape(&query.query))).unwrap();
    let found = search(data.config.git_repo.clone(), |path, meta| {
        if meta.is_file() {
            let title = ArticleTitle::from_path(&data.config.git_repo, path).ok()?;
            if re.is_match(title.as_ref()) {
                Some(title)
            } else {
                None
            }
        } else {
            None
        }
    })
    .await;

    Ok(render!(templates::SearchResults {
        query: &query.query,
        wiki: data.wiki(&account),
        results: &found,
    }))
}

