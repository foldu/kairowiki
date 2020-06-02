use crate::{data::Data, error::Error, filters::WikiArticle, templates};
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
    user_id: crate::user_storage::UserId,
) -> Result<impl Reply, Rejection> {
    super::unimplemented()
}

pub async fn history(data: Data, article: WikiArticle) -> Result<impl Reply, Rejection> {
    super::unimplemented()
}
