use crate::{article::WikiArticle, data::Data, user_storage::UserId};
use serde::{de::Deserializer, Deserialize};

#[derive(serde::Deserialize)]
pub struct PreviewMarkdown {
    markdown: String,
}

fn deserialize_oid<'de, D>(deserializer: D) -> Result<Option<git2::Oid>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => git2::Oid::from_str(&s).map(Some).map_err(D::Error::custom),
        None => Ok(None),
    }
}

#[derive(serde::Deserialize)]
pub struct EditSubmit {
    pub markdown: String,
    #[serde(deserialize_with = "deserialize_oid")]
    pub oid: Option<git2::Oid>,
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
    // FIXME:
    .unwrap();

    let markdown = tokio::fs::read_to_string(article.path.as_ref())
        .await
        .unwrap_or_else(|_| String::new());

    Ok(warp::reply::json(&crate::api::ArticleInfo {
        markdown,
        oid: oid.map(crate::api::HexEncode),
    }))
}
