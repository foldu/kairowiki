#[macro_export]
macro_rules! render {
    ($template:expr) => {
        render!(warp::http::StatusCode::OK, $template)
    };
    ($status:expr, $template:expr) => {
        warp::reply::with_status(
            warp::reply::html(askama::Template::render(&$template).unwrap()),
            $status,
        )
    };
}
