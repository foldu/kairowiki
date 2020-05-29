use crate::{data::Data, error::Error, forms, session::Sessions, templates, user_storage};
use askama::Template;
use std::path::Path;
use warp::{path::Tail, reject, Rejection, Reply};

pub fn template<T>(template: T) -> impl warp::Reply
where
    T: Template,
{
    warp::reply::html(template.render().unwrap())
}

pub async fn show_entry(data: Data, tail: Tail) -> Result<impl Reply, Rejection> {
    // TODO: add rendering cache
    let path = Path::new(tail.as_str()).with_extension("md");
    let md = match tokio::fs::read_to_string(data.repo_path.join(path)).await {
        Ok(cont) => cont,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(reject::not_found()),
        Err(e) => return Err(reject::custom(Error::Io(e))),
    };

    let mut rendered = String::new();
    pulldown_cmark::html::push_html(&mut rendered, pulldown_cmark::Parser::new(&md));

    // FIXME:
    Ok(warp::reply::html(
        templates::WikiPage {
            title: "fish",
            content: &rendered,
        }
        .render()
        .unwrap(),
    ))
}

pub async fn register(data: Data, form: forms::Register) -> Result<impl Reply, Rejection> {
    match data.user_storage.register(&form).await {
        Err(user_storage::Error::UserExists) => unimplemented!(),
        Err(user_storage::Error::EmailExists) => unimplemented!(),
        other => other.map_err(reject::custom).map(|_| {
            warp::reply::with_status(
                template(templates::RegisterRefresh {}),
                warp::http::StatusCode::CREATED,
            )
        }),
    }
}

pub async fn login(
    data: Data,
    sessions: Sessions,
    form: forms::Login,
) -> Result<impl warp::Reply, Rejection> {
    let user_id = data
        .user_storage
        .check_credentials(&form.name, &form.password)
        .await
        .map_err(reject::custom)?;

    let (uuid, expiry_time) = sessions.login(user_id).await;
    let cookie = cookie::CookieBuilder::new("warp-session", format!("{}", uuid))
        .max_age(expiry_time)
        .finish();

    Ok(warp::http::Response::builder()
        .status(301)
        .header("Set-Cookie", format!("{}", cookie))
        .header("Location", "/")
        .body("")
        .unwrap())
}

#[macro_export]
macro_rules! render_template {
    ($e:expr) => {
        || {
            fn inner() -> impl warp::Reply {
                Ok(crate::handlers::template($e))
            }
            inner()
        }
    };
}

pub async fn handle_rejection(
    err: Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let err = if err.is_not_found() {
        templates::Error::not_found()
    } else if let Some(error) = err.find::<Error>() {
        match error {
            Error::Io(_) => templates::Error::internal_server(),
        }
    }
    // TODO:
    //else if let Some(error) = err.find::<db::Error>() {
    //}
    else {
        // FIXME: should use display
        tracing::error!("{:?}", err);
        templates::Error::internal_server()
    };

    Ok(template(err))
}
