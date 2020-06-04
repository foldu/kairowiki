use crate::{
    article::WikiArticle, data::Data, error::Error, forms, templates, user_storage::UserId,
};
use warp::{
    reject::{self, Rejection},
    Reply,
};

fn render_markdown(src: &str) -> String {
    let mut rendered = String::new();
    let parser = pulldown_cmark::Parser::new_ext(src, pulldown_cmark::Options::ENABLE_TABLES);
    pulldown_cmark::html::push_html(&mut rendered, parser);
    rendered
}

pub async fn show_entry(data: Data, article: WikiArticle) -> Result<impl Reply, Rejection> {
    // TODO: add rendering cache
    let body = match tokio::fs::read_to_string(article.path.as_ref()).await {
        Ok(cont) => render_markdown(&cont),
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

pub async fn edit_post(
    data: Data,
    article: WikiArticle,
    user_id: UserId,
    new_article: forms::NewArticle,
) -> Result<impl Reply, Rejection> {
    let account = data
        .user_storage
        .fetch_account(user_id)
        .await
        .map_err(warp::reject::custom)?;

    tokio::task::block_in_place(move || {
        crate::git::commit_article(&data, &article, &account, &new_article)
    })
    .map_err(warp::reject::custom)?;

    Ok("")
}

pub async fn history(data: Data, article: WikiArticle) -> Result<impl Reply, Rejection> {
    super::unimplemented()
}
