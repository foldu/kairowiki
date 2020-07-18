use crate::{context::Context, templates, user_storage::UserAccount};

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    query: String,
}

pub async fn search_repo(
    ctx: Context,
    account: Option<UserAccount>,
    search_query: SearchQuery,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let found = tokio::task::block_in_place(|| ctx.index.search(&search_query.query, 10))
        // FIXME:
        .unwrap();

    Ok(render!(templates::SearchResults {
        query: &search_query.query,
        wiki: ctx.wiki(&account),
        results: &found,
    }))
}

