pub mod wiki;

use crate::{data::Data, error::Error, forms, session::Sessions, templates, user_storage};
use askama::Template;
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

pub fn unimplemented() -> Result<impl warp::Reply, Rejection> {
    Ok(warp::reply::with_status(
        "Currently not implemented",
        warp::http::StatusCode::BAD_REQUEST,
    ))
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    query: String,
}

pub async fn search(_query: SearchQuery) -> Result<impl warp::Reply, Rejection> {
    unimplemented()
}

pub async fn handle_rejection(
    err: Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    use crate::session;
    let mut response = warp::http::Response::builder();

    macro_rules! template_response {
        ($status:expr, $template:expr) => {
            response
                .status($status)
                .body($template.render().unwrap())
                .unwrap()
        };
    }

    Ok(if err.is_not_found() {
        template_response!(404, templates::Error::not_found())
    } else if let Some(error) = err.find::<Error>() {
        match error {
            Error::Io(_) => template_response!(500, templates::Error::internal_server()),
        }
    } else if let Some(error) = err.find::<crate::session::Error>() {
        response = response.status(301);
        match error {
            session::Error::CorruptedCookie => response
                .header("Set-Cookie", &crate::session::clear_browser_cookie())
                .header("Location", "/")
                .body("".to_string())
                .unwrap(),
            session::Error::SessionRequired { access_url } => {
                // FIXME: there must be a better way to do this
                // sadly the `url` crate can only do absolute urls
                let location = format!("/login?return_to={}", urlencoding::encode(access_url));
                response
                    .header("Location", location)
                    .body("".to_string())
                    .unwrap()
            }
        }
    } else {
        // FIXME: should use display
        tracing::error!("{:?}", err);
        template_response!(500, templates::Error::internal_server())
    })
}
