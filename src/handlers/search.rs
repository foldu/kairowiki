use crate::{context::Context, templates, user_storage::UserAccount};
use tantivy::{collector::TopDocs, query::QueryParser};

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    query: String,
}

pub async fn search_repo(
    ctx: Context,
    account: Option<UserAccount>,
    search_query: SearchQuery,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let searcher = tokio::task::block_in_place(|| ctx.index.reader.searcher());

    let title = ctx.index.schema.title;
    let content = ctx.index.schema.content;
    let query = QueryParser::for_index(searcher.index(), vec![title, content])
        .parse_query(&search_query.query)
        // FIXME: decide what to do if query is malformed
        .unwrap();

    // FIXME: when can this fail?
    let results = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

    // TODO: maybe add ellipsed contents of article?
    let mut found = Vec::with_capacity(10);
    for (_score, addr) in results {
        let doc = searcher.doc(addr).unwrap();
        found.push(doc.get_first(title).unwrap().text().unwrap().to_owned());
    }

    Ok(render!(templates::SearchResults {
        query: &search_query.query,
        wiki: ctx.wiki(&account),
        results: &found,
    }))
}

