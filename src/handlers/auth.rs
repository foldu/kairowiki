use crate::{data::Data, forms, session::Sessions, templates, user_storage};
use warp::{http::StatusCode, reject, Rejection, Reply};

pub async fn register_form(data: Data) -> Result<impl Reply, Rejection> {
    Ok(render!(templates::Register::new(data.wiki())))
}

pub async fn register(data: Data, form: forms::Register) -> Result<impl Reply, Rejection> {
    match data.user_storage.register(&form).await {
        Err(user_storage::Error::UserExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(data.wiki(), "User exists")
        )),
        Err(user_storage::Error::EmailExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(data.wiki(), "Email already registered")
        )),
        other => other
            .map_err(reject::custom)
            .map(|_| render!(StatusCode::CREATED, templates::RegisterRefresh {})),
    }
}

pub async fn login_form(data: Data) -> Result<impl warp::Reply, Rejection> {
    Ok(render!(templates::Login { wiki: data.wiki() }))
}

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    return_to: Option<String>,
}

pub async fn login(
    data: Data,
    sessions: Sessions,
    form: forms::Login,
    login_query: LoginQuery,
) -> Result<impl warp::Reply, Rejection> {
    let user_id = data
        .user_storage
        .check_credentials(&form.name, &form.password)
        .await
        .map_err(reject::custom)?;

    let (uuid, expiry_time) = sessions.login(user_id).await;
    let cookie = cookie::CookieBuilder::new(crate::session::COOKIE_NAME, format!("{}", uuid))
        .max_age(expiry_time)
        .finish();

    let location = match &login_query.return_to {
        Some(url) => url.as_str(),
        None => "/",
    };
    Ok(warp::http::Response::builder()
        .status(301)
        .header("Set-Cookie", format!("{}", cookie))
        .header("Location", location)
        .body("")
        .unwrap())
}

