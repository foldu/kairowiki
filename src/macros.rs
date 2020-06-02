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

macro_rules! routes {
    ($x:expr, $($y:expr),*) => { {
            let filter = boxed_on_debug!($x);
            $(
                let filter = boxed_on_debug!(filter.or($y));
            )*
            filter
    } }
}

#[cfg(debug_assertions)]
macro_rules! boxed_on_debug {
    ($x:expr) => {
        $x.boxed()
    };
}

#[cfg(not(debug_assertions))]
macro_rules! boxed_on_debug {
    ($x:expr) => {
        $x
    };
}
