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

macro_rules! sql_file {
    ($ident:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/", $ident, ".sql"))
    };
}

macro_rules! migration {
    ($ident:expr) => {
        crate::migrations::Migration {
            ident: $ident,
            migration: sql_file!($ident),
        }
    };
}
