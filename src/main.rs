#![feature(or_patterns)]
#[macro_use]
mod macros;
mod api;
mod article;
mod csp;
mod data;
mod file_storage;
mod forms;
mod git;
mod handlers;
mod markdown;
mod migrations;
mod relative_url;
mod serde;
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

    let search = warp::path!("search")
        .and(data_filter.clone())
        .and(warp::query())
        .and_then(handlers::search::search_repo);

    let home_url =
        warp::http::Uri::from_maybe_shared(format!("/wiki/{}", data.config.home_wiki_page.clone()))
            .unwrap();
    let home = root.map(move || warp::redirect(home_url.clone()));

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_article = crate::article::wiki_article(data.clone());
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_route = data_filter.clone().and(wiki_article.clone());

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

    let history = warp::path("history")
        .and(warp::get())
        .and(wiki_route)
        .and_then(handlers::wiki::history);

    let register_path = warp::path!("register");
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

    let login_path = warp::path!("login");
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
    let logout = warp::path!("logout")
        .and(warp::post())
        .and(login_required.clone())
        .and(sessions)
        .and_then(handlers::auth::logout);

    let file_storage = warp::path("storage");
    let upload = file_storage
        .and(warp::put())
        .and(data_filter.clone())
        .and(login_required.clone())
        .and(warp::filters::multipart::form().max_length(5 * (1 << 20)))
        .and_then(handlers::file_storage::upload);
    let serve_files = file_storage
        .and(warp::get())
        .and(warp::fs::dir(data.config.storage_path.clone()));

    let api = warp::path("api");
    let put_api = api
        .and(warp::body::content_length_limit(2 * (1 << 20)))
        .and(warp::put());
    let preview = put_api
        .and(warp::path!("preview"))
        .and(data_filter.clone())
        .and(login_required.clone())
        .and(warp::body::json())
        .and_then(handlers::api::preview);
    let edit_submit = put_api
        .and(warp::path("edit"))
        .and(data_filter.clone())
        .and(wiki_article.clone())
        .and(login_required.clone())
        .and(warp::body::json())
        .and_then(handlers::api::edit_submit);
    let article_info = api
        .and(warp::path("article_info"))
        .and(data_filter.clone())
        .and(wiki_article.clone())
        .and(warp::get())
        .and_then(handlers::api::article_info);

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
        upload,
        serve_files,
        preview,
        edit_submit,
        article_info
    };
    //let routes = routes.or();

    let domain = data.config.domain.as_ref().cloned().unwrap_or_else(|| {
        url::Url::parse(&format!("http://localhost:{}", data.config.port)).unwrap()
    });

    let cors = warp::cors()
        .allow_methods(vec!["GET", "PUT", "POST", "HEAD"])
        .allow_credentials(true)
        .allow_origin(domain.as_str())
        .allow_origin("https://cdnjs.cloudflare.com")
        .build();

    let csp = csp::Builder::new()
        .script_sources(vec![domain.as_str(), "https://cdnjs.cloudflare.com"])
        .worker_source("data:")
        .build();

    let routes = routes
        .recover(handlers::handle_rejection)
        .with(cors)
        .with(csp);

    let term = signal(SignalKind::terminate()).unwrap();
    let int = signal(SignalKind::interrupt()).unwrap();
    let shutdown = async move {
        stream::select(term, int).next().await;
    };

    let addr = std::net::SocketAddr::new(data.config.ip_addr, data.config.port);
    let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, shutdown);

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
        let mut chain = e.chain();
        if let Some(head) = chain.next() {
            eprintln!("{}", head);
        }
        for cause in chain {
            eprintln!("Caused by: {}", cause);
        }
        std::process::exit(1);
    }
}
