pub mod auth;
pub mod file_storage;
pub mod search;
pub mod wiki;

use crate::{error::Error, relative_url::RelativeUrl, templates};
use warp::{http::StatusCode, Rejection};

pub fn unimplemented() -> Result<impl warp::Reply, Rejection> {
    Ok(warp::reply::with_status(
        "Currently not implemented",
        warp::http::StatusCode::BAD_REQUEST,
    ))
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
                .body(askama::Template::render(&$template).unwrap())
                .unwrap()
        };
    }

    Ok(if err.is_not_found() {
        template_response!(StatusCode::NOT_FOUND, templates::Error::not_found())
    } else if let Some(error) = err.find::<Error>() {
        match error {
            Error::Io(_) => template_response!(
                StatusCode::INTERNAL_SERVER_ERROR,
                templates::Error::internal_server()
            ),
        }
    } else if let Some(error) = err.find::<crate::session::Error>() {
        response = response.status(StatusCode::PERMANENT_REDIRECT);
        match error {
            session::Error::CorruptedCookie => response
                .header("Set-Cookie", crate::session::ClearCookie)
                .header("Location", "/")
                .body("".to_string())
                .unwrap(),
            session::Error::SessionRequired { access_url } => {
                let location = RelativeUrl::builder("/login")
                    .unwrap()
                    .query("return_to", access_url)
                    .build();

                response
                    .header("Location", location.as_ref())
                    .body("".to_string())
                    .unwrap()
            }
        }
    } else {
        // FIXME: should use display
        tracing::error!("{:?}", err);
        template_response!(
            StatusCode::INTERNAL_SERVER_ERROR,
            templates::Error::internal_server()
        )
    })
}
