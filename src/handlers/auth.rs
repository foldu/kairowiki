use crate::{
    data::Data,
    forms,
    session::Sessions,
    templates,
    user_storage::{self, UserId},
};
use warp::{http::StatusCode, reject, Rejection, Reply};

pub async fn register_form(data: Data) -> Result<impl Reply, Rejection> {
    // TODO: better error message about registration being disabled/not supported
    Ok(if data.registration_possible() {
        render!(templates::Register::new(data.wiki()))
    } else {
        render!(
            warp::http::StatusCode::NOT_IMPLEMENTED,
            templates::Error::not_implemented()
        )
    })
}

pub async fn register(data: Data, form: forms::Register) -> Result<impl Reply, Rejection> {
    if !data.registration_possible() {
        // TODO: same as in register_form
        return Ok(render!(
            warp::http::StatusCode::NOT_IMPLEMENTED,
            templates::Error::not_implemented()
        ));
    }

    match data.user_storage.register(&form).await {
        Err(user_storage::Error::UserExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(data.wiki(), "User exists")
        )),
        Err(user_storage::Error::EmailExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(data.wiki(), "Email already registered")
        )),
        other => other.map_err(reject::custom).map(|_| {
            render!(
                StatusCode::CREATED,
                templates::RegisterRefresh { wiki: data.wiki() }
            )
        }),
    }
}

pub async fn login_form(data: Data) -> Result<impl warp::Reply, Rejection> {
    Ok(render!(templates::Login {
        wiki: data.wiki(),
        registration_enabled: data.registration_possible(),
        error: None
    }))
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
    use crate::user_storage::Error::*;
    let user_id = match data
        .user_storage
        .check_credentials(&form.name, &form.password)
        .await
    {
        Err(e @ UserDoesNotExist | e @ InvalidPassword) => {
            return Ok(warp::http::Response::builder()
                .status(warp::http::StatusCode::FORBIDDEN)
                .body(
                    askama::Template::render(&templates::Login::new(&data, Some(&e.to_string())))
                        .unwrap(),
                )
                .unwrap());
        }
        cred => cred.map_err(reject::custom),
    }?;

    let session = sessions.login(user_id).await;

    let location = match &login_query.return_to {
        Some(url) => url.as_str(),
        None => "/",
    };
    Ok(warp::http::Response::builder()
        .status(301)
        .header("Set-Cookie", session)
        .header("Location", location)
        .body("".to_string())
        .unwrap())
}

pub async fn logout(
    user_id: UserId,
    sessions: Sessions,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    sessions.logout(user_id).await;
    Ok(warp::http::Response::builder()
        .status(StatusCode::PERMANENT_REDIRECT)
        .header("Set-Cookie", crate::session::ClearCookie)
        .header("Location", "/")
        .body("".to_string())
        .unwrap())
}
