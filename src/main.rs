mod data;
mod error;
mod forms;
mod handlers;
mod password;
mod session;
mod templates;
mod user_storage;

use tokio::runtime;
use warp::{http::Uri, Filter};

async fn run() -> Result<(), anyhow::Error> {
    let _subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let data = data::Data::from_env().await?;
    let port = data.port;
    let static_ = warp::path("static").and(warp::fs::dir(&data.static_dir));
    let data_filter = warp::any().map(move || data.clone());
    let form_size_limit = warp::body::content_length_limit(1 << 10);
    let sessions = session::Sessions::new(std::time::Duration::from_secs(5 * 60));
    let sessions = warp::any().map(move || sessions.clone());

    let home = warp::get().and(warp::path::end()).map(|| "Home page");

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_entries = wiki
        .and(data_filter.clone())
        .and(warp::path::tail())
        .and_then(handlers::show_entry);

    let register_path = warp::path("register").and(warp::path::end());
    let register_form = register_path
        .and(warp::get())
        .map(render_template!(templates::Register {}));
    let register_post = register_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and_then(handlers::register);

    let login_path = warp::path("login").and(warp::path::end());
    let login_form = login_path
        .and(warp::get())
        .map(render_template!(templates::Login {}));
    let login_post = login_path
        .and(warp::post())
        .and(data_filter.clone())
        .and(sessions.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .and_then(handlers::login);

    let routes = home
        .or(wiki_home)
        .or(wiki_entries)
        .or(register_form)
        .or(register_post)
        .or(login_form)
        .or(login_post)
        .or(static_);

    warp::serve(routes.recover(handlers::handle_rejection))
        .run(([0, 0, 0, 0], port))
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
