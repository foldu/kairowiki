#[macro_use]
mod macros;
mod data;
mod error;
mod filters;
mod forms;
mod handlers;
mod session;
mod templates;
mod user_storage;

use tokio::runtime;
use warp::{http::Uri, Filter};

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

async fn run() -> Result<(), anyhow::Error> {
    let _subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    // FIXME: clean this up
    let data = data::Data::from_env().await?;
    let static_ = warp::path("static").and(warp::fs::dir(data.config.static_dir.clone()));

    let data_ = data.clone();
    let data_filter = warp::any().map(move || data_.clone());
    let form_size_limit = warp::body::content_length_limit(1 << 10);
    let sessions = session::Sessions::new(std::time::Duration::from_secs(5 * 60));
    let login_required = session::login_required(sessions.clone());
    let sessions = warp::any().map(move || sessions.clone());

    let root = warp::get().and(warp::path::end());
    let search = root.and(warp::query()).and_then(handlers::search);
    let home_url =
        warp::http::Uri::from_maybe_shared(format!("/wiki/{}", data.config.home_wiki_page.clone()))
            .unwrap();
    let home = root.map(move || warp::redirect(home_url.clone()));

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_article = wiki
        .and(data_filter.clone())
        .and(crate::filters::wiki_article(data.clone()));
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_entries = wiki_article
        .clone()
        .and(warp::path::end())
        .and_then(handlers::wiki::show_entry);
    let wiki_edit = wiki_article
        .clone()
        .and(warp::path("edit"))
        .and(warp::path::end())
        .and(login_required)
        .and_then(handlers::wiki::edit);
    let wiki_history = wiki_article
        .clone()
        .and(warp::path("history"))
        .and(warp::path::end())
        .and_then(handlers::wiki::history);

    let register_path = warp::path("register").and(warp::path::end());
    let register_form = register_path
        .and(warp::get())
        .and(data_filter.clone())
        .and_then(handlers::register_form);
    let register_post = register_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and_then(handlers::register);

    let login_path = warp::path("login").and(warp::path::end());
    let login_form = login_path
        .and(warp::get())
        .and(data_filter.clone())
        .and_then(handlers::login_form);
    let login_post = login_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(sessions.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and(warp::query())
        .and_then(handlers::login);

    let routes = routes! {
        home,
        wiki_home,
        wiki_entries,
        register_form,
        register_post,
        login_form,
        login_post,
        search,
        static_,
        wiki_edit,
        wiki_history
    };
    let routes = routes.recover(handlers::handle_rejection);

    warp::serve(routes)
        .run(([0, 0, 0, 0], data.config.port))
        .await;

    Ok(())
}

fn main() {
    let mut rt = runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()
        .unwrap();

    if let Err(e) = rt.block_on(run()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
