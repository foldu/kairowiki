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
        tokio::task::block_in_place(|| {
            ctx.index
                .update_article(&article.title, &edit.markdown)
                // FIXME:
                .unwrap()
        });
    }

    Ok(warp::reply::json(&resp))
}

pub async fn article_info(
    ctx: Context,
    article: WikiArticle,
) -> Result<impl warp::Reply, warp::Rejection> {
    let info = tokio::task::block_in_place(|| -> Result<_, crate::git::Error> {
        let repo = ctx.repo.read()?;
        let head_commit_id = repo.head()?.target().unwrap();
        let article_info = repo.article_at_rev(head_commit_id, &article.path)?;

        Ok((article_info, head_commit_id))
    })
    .map_err(warp::reject::custom)?;

    let (oid, markdown) = match info.0 {
        Some((oid, content)) => (Some(oid), content),
        None => (None, String::new()),
    };

    Ok(warp::reply::json(&crate::api::ArticleInfo {
        markdown,
        oid,
        rev: crate::serde::Oid(info.1),
    }))
}
