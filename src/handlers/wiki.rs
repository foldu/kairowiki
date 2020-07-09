use crate::{article::WikiArticle, data::Data, serde::Oid, templates, user_storage::UserAccount};
use warp::{
    reject::{self, Rejection},
    Reply,
};

#[derive(serde::Deserialize)]
pub struct EntryQuery {
    rev: Option<Oid>,
}

pub async fn show_entry(
    data: Data,
    article: WikiArticle,
    account: Option<UserAccount>,
    query: EntryQuery,
) -> Result<impl Reply, Rejection> {
    // TODO: add rendering cache
    let body = match query.rev {
        None => match article.read_to_string().await {
            Ok(cont) => tokio::task::block_in_place(|| data.markdown_renderer.render(&cont)),
            Err(crate::article::Error::DoesNotExist) => format!(
                "Article with title {} not found, click on edit to create it",
                article.title.as_ref()
            ),
            Err(e) => return Err(reject::custom(e)),
        },
        Some(rev) => {
            tokio::task::block_in_place(|| data.repo.read()?.article_at_rev(rev.0, &article.path))
                .map_err(warp::reject::custom)?
                .map(|cont| data.markdown_renderer.render(&cont))
                .unwrap_or_else(|| {
                    // FIXME: maybe return a 404 error page here instead?
                    format!(
                        "Article with name {} and commit id {} not found",
                        article.title.as_ref(),
                        rev.0
                    )
                })
        }
    };

    Ok(render!(templates::WikiPage {
        title: &article.title,
        content: &body,
        wiki: data.wiki(&account),
    }))
}

pub async fn edit(
    data: Data,
    article: WikiArticle,
    account: UserAccount,
) -> Result<impl Reply, Rejection> {
    Ok(render!(templates::WikiEdit {
        wiki: data.wiki(&Some(account)),
        title: article.title.as_ref()
    }))
}

pub async fn history(
    data: Data,
    article: WikiArticle,
    account: Option<UserAccount>,
) -> Result<impl Reply, Rejection> {
    let history =
        tokio::task::block_in_place(|| data.repo.read().and_then(|repo| repo.history(&article)))
            .map_err(warp::reject::custom)?;

    Ok(render!(templates::History {
        wiki: data.wiki(&account),
        title: article.title.as_ref(),
        history: &history,
    }))
}

