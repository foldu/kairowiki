use crate::{
    api::{EditSubmit, PreviewMarkdown, RenderedMarkdown},
    article::WikiArticle,
    data::Data,
    user_storage::UserId,
};

pub async fn preview(
    data: Data,
    _user_id: UserId,
    request: PreviewMarkdown,
) -> Result<impl warp::Reply, warp::Rejection> {
    let md = data.markdown_renderer.render(&request.markdown);
    Ok(warp::reply::json(&RenderedMarkdown { rendered: md }))
}

pub async fn edit_submit(
    data: Data,
    article: WikiArticle,
    user_id: UserId,
    edit: EditSubmit,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account = data
        .user_storage
        .fetch_account(user_id)
        .await
        .map_err(warp::reject::custom)?;

    let repo = data.repo.write().await;

    let resp = tokio::task::block_in_place(move || repo.commit_article(&article, &account, edit))
        .map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&resp))
}

pub async fn article_info(
    data: Data,
    article: WikiArticle,
) -> Result<impl warp::Reply, warp::Rejection> {
    let oid: Option<git2::Oid> = tokio::task::block_in_place(|| {
        data.repo
            .read()
            .and_then(|repo| repo.get_current_oid_for_article(&article))
    })
    .map_err(warp::reject::custom)?;

    let markdown = tokio::fs::read_to_string(article.path.as_ref())
        .await
        .unwrap_or_else(|_| String::new());

    Ok(warp::reply::json(&crate::api::ArticleInfo {
        markdown,
        oid: oid.map(crate::serde::HexEncode),
    }))
}
