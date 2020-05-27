mod data;
mod error;
mod handlers;
mod templates;

use tokio::runtime;
use warp::{http::Uri, Filter};

async fn run() -> Result<(), anyhow::Error> {
    let _subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let data = data::Data::from_env().await?;
    let data_filter = warp::any().map(move || data.clone());

    let home = warp::get().and(warp::path::end()).map(|| "Home page");

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_entries = wiki
        .and(data_filter)
        .and(warp::path::tail())
        .and_then(handlers::show_entry);

    let routes = home.or(wiki_home).or(wiki_entries);
    warp::serve(routes.recover(handlers::handle_rejection))
        .run(([0, 0, 0, 0], 8080))
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
