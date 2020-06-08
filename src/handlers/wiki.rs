use crate::{article::WikiArticle, data::Data, error::Error, templates, user_storage::UserId};
use warp::{
    reject::{self, Rejection},
    Reply,
};

pub async fn show_entry(data: Data, article: WikiArticle) -> Result<impl Reply, Rejection> {
    // TODO: add rendering cache
    let body = match tokio::fs::read_to_string(article.path.as_ref()).await {
        Ok(cont) => tokio::task::block_in_place(|| crate::markdown::render(&cont)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => format!(
            "Article with title {} not found, click on edit to create it",
            article.title.as_ref()
        ),
        Err(e) => return Err(reject::custom(Error::Io(e))),
    };

    Ok(render!(templates::WikiPage {
        title: &article.title,
        content: &body,
        wiki: data.wiki(),
    }))
}

pub async fn edit(
    data: Data,
    article: WikiArticle,
    user_id: UserId,
) -> Result<impl Reply, Rejection> {
    let markdown = tokio::fs::read_to_string(article.path.as_ref())
        .await
        .unwrap_or_else(|_| String::new());

    Ok(render!(templates::WikiEdit {
        title: &article.title,
        wiki: data.wiki(),
        markdown: &markdown,
    }))
}

pub async fn history(data: Data, article: WikiArticle) -> Result<impl Reply, Rejection> {
    super::unimplemented()
}
