use crate::{
    article::WikiArticle, context::Context, relative_url::RelativeUrl, serde::Oid, templates,
    user_storage::UserAccount,
};
use warp::{reject::Rejection, Reply};

#[derive(serde::Deserialize)]
pub struct EntryQuery {
    rev: Option<Oid>,
}

pub fn add_article_form(ctx: Context, account: UserAccount) -> impl Reply {
    render!(templates::AddArticle {
        wiki: ctx.wiki(&Some(account)),
    })
}

pub fn add_article(
    _ctx: Context,
    _account: UserAccount,
    add_article_form: crate::forms::AddArticle,
) -> impl Reply {
    let url = RelativeUrl::builder("/edit")
        .unwrap()
        .element(&add_article_form.title)
        .build();
    println!("{}", url.as_ref());
    let url = warp::http::Uri::from_maybe_shared(url.as_ref().to_owned()).unwrap();
    Ok(warp::redirect(url))
}

pub async fn show_entry(
    ctx: Context,
    article: WikiArticle,
    account: Option<UserAccount>,
    query: EntryQuery,
) -> Result<impl Reply, Rejection> {
    // TODO: add rendering cache
    let body = match query.rev {
        None => tokio::task::block_in_place(|| match ctx.index.get_article(&article.title) {
            Some(cont) => ctx.markdown_renderer.render(&cont),
            None => format!(
                "Article with title {} not found, click on edit to create it",
                article.title.as_ref()
            ),
        }),
        Some(rev) => {
            tokio::task::block_in_place(|| ctx.repo.read()?.article_at_rev(rev.0, &article.path))
                .map_err(warp::reject::custom)?
                .map(|cont| ctx.markdown_renderer.render(&cont))
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
        wiki: ctx.wiki(&account),
    }))
}

pub fn edit(ctx: Context, article: WikiArticle, account: UserAccount) -> impl Reply {
    render!(templates::WikiEdit {
        wiki: ctx.wiki(&Some(account)),
        title: article.title.as_ref()
    })
}

pub async fn history(
    ctx: Context,
    article: WikiArticle,
    account: Option<UserAccount>,
) -> Result<impl Reply, Rejection> {
    let history =
        tokio::task::block_in_place(|| ctx.repo.read().and_then(|repo| repo.history(&article)))
            .map_err(warp::reject::custom)?;

    Ok(render!(templates::History {
        wiki: ctx.wiki(&account),
        title: article.title.as_ref(),
        history: &history,
    }))
}
