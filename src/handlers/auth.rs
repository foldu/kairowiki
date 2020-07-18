use crate::{
    context::Context,
    forms,
    session::Sessions,
    templates,
    user_storage::{self, UserAccount},
};
use warp::{http::StatusCode, reject, Rejection, Reply};

pub async fn register_form(
    ctx: Context,
    account: Option<UserAccount>,
) -> Result<impl Reply, Rejection> {
    // TODO: better error message about registration being disabled/not supported
    Ok(if ctx.registration_possible() {
        render!(templates::Register::new(ctx.wiki(&account)))
    } else {
        render!(
            warp::http::StatusCode::NOT_IMPLEMENTED,
            templates::Error::not_implemented()
        )
    })
}

pub async fn register(
    ctx: Context,
    account: Option<UserAccount>,
    form: forms::Register,
) -> Result<impl Reply, Rejection> {
    if !ctx.registration_possible() {
        // TODO: same as in register_form
        return Ok(render!(
            warp::http::StatusCode::NOT_IMPLEMENTED,
            templates::Error::not_implemented()
        ));
    }

    let wiki = ctx.wiki(&account);
    if form.password != form.password_check {
        return Ok(render!(
            StatusCode::BAD_REQUEST,
            templates::Register::error(wiki, "Password repeat does not match password")
        ));
    }

    match ctx.user_storage.register(&form).await {
        Err(user_storage::Error::UserExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(wiki, "User exists")
        )),
        Err(user_storage::Error::EmailExists) => Ok(render!(
            StatusCode::CONFLICT,
            templates::Register::error(wiki, "Email already registered")
        )),
        other => other
            .map_err(reject::custom)
            .map(|_| render!(StatusCode::CREATED, templates::RegisterRefresh { wiki })),
    }
}

pub async fn login_form(
    ctx: Context,
    account: Option<UserAccount>,
) -> Result<impl warp::Reply, Rejection> {
    Ok(render!(templates::Login {
        wiki: ctx.wiki(&account),
        registration_enabled: ctx.registration_possible(),
        error: None
    }))
}

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    return_to: Option<String>,
}

pub async fn login(
    ctx: Context,
    account: Option<UserAccount>,
    sessions: Sessions,
    form: forms::Login,
    login_query: LoginQuery,
) -> Result<impl warp::Reply, Rejection> {
    use crate::user_storage::Error::*;
    let account = match ctx
        .user_storage
        .check_credentials(&form.name, &form.password)
        .await
    {
        Err(e @ UserDoesNotExist | e @ InvalidPassword) => {
            return Ok(warp::http::Response::builder()
                .status(warp::http::StatusCode::FORBIDDEN)
                .body(
                    askama::Template::render(&templates::Login::new(
                        &ctx,
                        &account,
                        Some(&e.to_string()),
                    ))
                    .unwrap(),
                )
                .unwrap());
        }
        cred => cred.map_err(reject::custom),
    }?;

    let session = sessions.login(account).await;

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
    account: UserAccount,
    sessions: Sessions,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    sessions.logout(account.id).await;
    Ok(warp::http::Response::builder()
        .status(301)
        .header("Set-Cookie", crate::session::ClearCookie)
        .header("Location", "/")
        .body("".to_string())
        .unwrap())
}
