#[macro_use]
mod macros;
mod article;
mod data;
mod error;
mod forms;
mod git;
mod handlers;
mod migrations;
mod session;
mod sqlite;
mod templates;
mod user_storage;

use futures_util::stream::{self, StreamExt};
use tokio::{
    runtime,
    signal::unix::{signal, SignalKind},
};
use warp::{http::Uri, Filter};

async fn run() -> Result<(), anyhow::Error> {
    init_logging();

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

    let search = warp::path("search")
        .and(warp::path::end())
        .and(data_filter.clone())
        .and(warp::query())
        .and_then(handlers::search::search_repo);

    let home_url =
        warp::http::Uri::from_maybe_shared(format!("/wiki/{}", data.config.home_wiki_page.clone()))
            .unwrap();
    let home = root.map(move || warp::redirect(home_url.clone()));

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_route = data_filter
        .clone()
        .and(crate::article::wiki_article(data.clone()));

    let wiki_entries = wiki
        .and(wiki_route.clone())
        .and_then(handlers::wiki::show_entry);

    let edit_route = warp::path("edit")
        .and(wiki_route.clone())
        .and(login_required.clone());
    let edit = edit_route
        .clone()
        .and(warp::get())
        .and_then(handlers::wiki::edit);
    let edit_post = edit_route
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::wiki::edit_post);

    let history = warp::path("history")
        .and(warp::get())
        .and(wiki_route)
        .and_then(handlers::wiki::history);

    let register_path = warp::path("register").and(warp::path::end());
    let register_form = register_path
        .and(warp::get())
        .and(data_filter.clone())
        .and_then(handlers::auth::register_form);
    let register_post = register_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and_then(handlers::auth::register);

    let login_path = warp::path("login").and(warp::path::end());
    let login_form = login_path
        .and(warp::get())
        .and(data_filter.clone())
        .and_then(handlers::auth::login_form);
    let login_post = login_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(sessions.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and(warp::query())
        .and_then(handlers::auth::login);
    let logout = warp::path("logout")
        .and(warp::path::end())
        .and(warp::post())
        .and(login_required)
        .and(sessions)
        .and_then(handlers::auth::logout);

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
        edit,
        history,
        logout,
        edit_post
    };
    let routes = routes.recover(handlers::handle_rejection);

    let term = signal(SignalKind::terminate()).unwrap();
    let int = signal(SignalKind::interrupt()).unwrap();
    let shutdown = async move {
        stream::select(term, int).next().await;
    };

    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], data.config.port), shutdown);

    tracing::info!("Listening on http://{}", addr);

    server.await;

    Ok(())
}

fn init_logging() {
    // FIXME: hack for default log level=info
    match std::env::var_os("RUST_LOG") {
        Some(_) => (),
        None => {
            std::env::set_var("RUST_LOG", "info");
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init()
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
