use crate::{
    api::{EditSubmit, PreviewMarkdown, RenderedMarkdown},
    article::WikiArticle,
    data::Data,
    user_storage::UserAccount,
};

pub async fn preview(
    data: Data,
    _account: UserAccount,
    request: PreviewMarkdown,
) -> Result<impl warp::Reply, warp::Rejection> {
    let md = data.markdown_renderer.render(&request.markdown);
    Ok(warp::reply::json(&RenderedMarkdown { rendered: md }))
}

pub async fn edit_submit(
    data: Data,
    article: WikiArticle,
    account: UserAccount,
    edit: EditSubmit,
) -> Result<impl warp::Reply, warp::Rejection> {
    let repo = data.repo.write().await;

    let resp = tokio::task::block_in_place(move || repo.commit_article(&article, &account, edit))
        .map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&resp))
}

pub async fn article_info(
    data: Data,
    article: WikiArticle,
) -> Result<impl warp::Reply, warp::Rejection> {
    let info = tokio::task::block_in_place(|| -> Result<_, crate::git::Error> {
        let repo = data.repo.read()?;
        let head = repo.head()?;
        let oid = repo.oid_for_article(&head, &article)?;

        let commit_id = head.peel_to_commit()?.id();
        Ok((oid, commit_id))
    })
    .map_err(warp::reject::custom)?;

    let markdown = tokio::fs::read_to_string(article.path.as_ref())
        .await
        .unwrap_or_else(|_| String::new());

    Ok(warp::reply::json(&crate::api::ArticleInfo {
        markdown,
        oid: info.0.map(crate::serde::Oid),
        rev: crate::serde::Oid(info.1),
    }))
}

