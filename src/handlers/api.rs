use crate::{article::WikiArticle, data::Data, user_storage::UserId};

#[derive(serde::Deserialize)]
pub struct PreviewMarkdown {
    markdown: String,
}

#[derive(serde::Deserialize)]
pub struct EditSubmit {
    pub markdown: String,
}

pub async fn preview(
    data: Data,
    _user_id: UserId,
    request: PreviewMarkdown,
) -> Result<impl warp::Reply, warp::Rejection> {
    #[derive(serde::Serialize)]
    struct RenderedMarkdown {
        rendered: String,
    }

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

    tokio::task::block_in_place(move || {
        crate::git::commit_article(&data.config.git_repo, &article, &account, &edit)
    })
    .map_err(warp::reject::custom)?;

    Ok("")
}
