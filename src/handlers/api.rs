use crate::{
    api::{EditSubmit, PreviewMarkdown, RenderedMarkdown},
    article::WikiArticle,
    context::Context,
    user_storage::UserAccount,
};

pub async fn preview(
    ctx: Context,
    _account: UserAccount,
    request: PreviewMarkdown,
) -> Result<impl warp::Reply, warp::Rejection> {
    let md = ctx.markdown_renderer.render(&request.markdown);
    Ok(warp::reply::json(&RenderedMarkdown { rendered: md }))
}

pub async fn edit_submit(
    ctx: Context,
    article: WikiArticle,
    account: UserAccount,
    edit: EditSubmit,
) -> Result<impl warp::Reply, warp::Rejection> {
    let repo = ctx.repo.write().await;

    let resp = tokio::task::block_in_place(|| repo.commit_article(&article.path, &account, &edit))
        .map_err(warp::reject::custom)?;

    if let crate::api::Commit::NoConflict = resp {
        ctx.index
            .update_article(&article.title, &edit.markdown)
            .await
            // FIXME:
            .unwrap();
    }

    Ok(warp::reply::json(&resp))
}

pub async fn article_info(
    ctx: Context,
    article: WikiArticle,
) -> Result<impl warp::Reply, warp::Rejection> {
    let info = tokio::task::block_in_place(|| -> Result<_, crate::git::Error> {
        let repo = ctx.repo.read()?;
        let head = repo.head()?;
        let oid = repo.oid_for_article(&head, &article.path)?;

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
