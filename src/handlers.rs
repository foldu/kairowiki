use crate::{data::Data, error::Error, templates};
use askama::Template;
use std::path::Path;
use warp::{path::Tail, reject, Rejection, Reply};

fn template<T>(template: T) -> impl warp::Reply
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

pub async fn handle_rejection(
    err: Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let err = if err.is_not_found() {
        templates::Error::not_found()
    } else if let Some(error) = err.find::<Error>() {
        match error {
            Error::Io(_) => templates::Error::internal_server(),
        }
    } else {
        templates::Error::internal_server()
    };

    Ok(template(err))
}
