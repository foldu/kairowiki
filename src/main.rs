#![feature(or_patterns, try_blocks)]
#[macro_use]
mod macros;
mod api;
mod article;
mod context;
mod csp;
mod file_storage;
mod forms;
mod git;
mod handlers;
mod index;
mod ipc;
mod markdown;
mod migrations;
mod post_receive_hook;
mod relative_url;
mod serde;
mod session;
mod sqlite;
mod templates;
mod user_storage;

use anyhow::Context;
use futures_util::stream::{self, StreamExt};
use tokio::{
    runtime,
    signal::unix::{signal, SignalKind},
};
use warp::{http::Uri, Filter};

async fn run() -> Result<(), anyhow::Error> {
    init_logging();

    // FIXME: clean this up
    let ctx = context::Context::from_env().await?;
    let static_ = warp::path("static").and(warp::fs::dir(ctx.config.static_dir.clone()));
    // TODO: move this to context
    let mut update_stream = ipc::listen(ipc::SOCK_PATH).context("Could not listen on unix sock")?;
    tokio::task::spawn({
        let ctx = ctx.clone();
        async move {
            while let Some(_update) = update_stream.next().await {
                let ret = tokio::task::block_in_place(|| -> Result<_, anyhow::Error> {
                    tracing::info!("Detected push");
                    let repo = ctx.repo.read()?;
                    ctx.index.rebuild(&repo)?;
                    Ok(())
                });
                if let Err(e) = ret {
                    tracing::error!("Failed to rebuild index: {}", e);
                }
            }
        }
    });

    let ctx_filter = warp::any().map({
        let ctx = ctx.clone();
        move || ctx.clone()
    });
    let form_size_limit = warp::body::content_length_limit(1 << 10);
    let sessions = session::Sessions::new(std::time::Duration::from_secs(5 * 60));
    let login_required = session::login_required(sessions.clone());
    let login_optional = session::login_optional(sessions.clone());
    let sessions = warp::any().map(move || sessions.clone());

    let root = warp::get().and(warp::path::end());

    let search = warp::path!("search")
        .and(ctx_filter.clone())
        .and(login_optional.clone())
        .and(warp::query())
        .and_then(handlers::search::search_repo);

    let home_url =
        warp::http::Uri::from_maybe_shared(format!("/wiki/{}", ctx.config.home_wiki_page.clone()))
            .unwrap();
    let home = root.map(move || warp::redirect(home_url.clone()));

    let wiki = warp::get().and(warp::path("wiki"));
    let wiki_article = crate::article::wiki_article();
    let wiki_home = wiki
        .and(warp::path::end())
        .map(|| warp::redirect(Uri::from_static("/")));
    let wiki_route = ctx_filter.clone().and(wiki_article.clone());

    let wiki_entries = wiki
        .and(wiki_route.clone())
        .and(login_optional.clone())
        .and(warp::query())
        .and_then(handlers::wiki::show_entry);

    let edit_route = warp::path("edit")
        .and(wiki_route.clone())
        .and(login_required.clone());
    let edit = edit_route
        .clone()
        .and(warp::get())
        .map(handlers::wiki::edit);

    let history = warp::path("history")
        .and(warp::get())
        .and(wiki_route)
        .and(login_optional.clone())
        .and_then(handlers::wiki::history);

    let register_path = warp::path!("register");
    let register_form = register_path
        .and(warp::get())
        .and(ctx_filter.clone())
        .and(login_optional.clone())
        .and_then(handlers::auth::register_form);
    let register_post = register_path
        .and(warp::post())
        .and(ctx_filter.clone())
        .and(form_size_limit)
        .and(login_optional.clone())
        .and(warp::filters::body::form())
        .and_then(handlers::auth::register);

    let login_path = warp::path!("login");
    let login_form = login_path
        .and(warp::get())
        .and(ctx_filter.clone())
        .and(login_optional.clone())
        .and_then(handlers::auth::login_form);
    let login_post = login_path
        .and(warp::post())
        .and(ctx_filter.clone())
        .and(login_optional)
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
        .and(ctx_filter.clone())
        .and(login_required.clone())
        .and(warp::filters::multipart::form().max_length(5 * (1 << 20)))
        .and_then(handlers::file_storage::upload);
    let serve_files = file_storage
        .and(warp::get())
        .and(warp::fs::dir(ctx.config.storage_path.clone()));

    let api = warp::path("api");
    let put_api = api
        .and(warp::body::content_length_limit(2 * (1 << 20)))
        .and(warp::put());
    let preview = put_api
        .and(warp::path!("preview"))
        .and(ctx_filter.clone())
        .and(login_required.clone())
        .and(warp::body::json())
        .and_then(handlers::api::preview);
    let edit_submit = put_api
        .and(warp::path("edit"))
        .and(ctx_filter.clone())
        .and(wiki_article.clone())
        .and(login_required.clone())
        .and(warp::body::json())
        .and_then(handlers::api::edit_submit);
    let article_info = api
        .and(warp::path("article_info"))
        .and(ctx_filter.clone())
        .and(wiki_article.clone())
        .and(warp::get())
        .and_then(handlers::api::article_info);

    let add_article = warp::path!("add_article").and(ctx_filter.clone());
    let add_article_form = add_article
        .clone()
        .and(warp::get())
        .and(login_required.clone())
        .map(handlers::wiki::add_article_form);
    let add_article = add_article
        .and(warp::post())
        .and(login_required.clone())
        .and(form_size_limit)
        .and(warp::filters::body::form())
        .map(handlers::wiki::add_article);

    let user = login_form
        .boxed()
        .or(register_form.boxed().or(register_post.boxed()))
        .or(login_post.boxed().or(logout.boxed()));
    let wiki = wiki_home
        .boxed()
        .or(wiki_entries.boxed().or(edit.boxed()))
        .or(history.boxed().or(search.boxed()));
    let files = static_.boxed().or(upload.boxed().or(serve_files.boxed()));
    let api = preview
        .boxed()
        .or(article_info.boxed().or(edit_submit.boxed()));
    let add_article = add_article.boxed().or(add_article_form.boxed());

    let routes = home.or(user.or(wiki)).or(api.or(files)).or(add_article);

    let domain = ctx.config.domain.as_ref().cloned().unwrap_or_else(|| {
        url::Url::parse(&format!("http://localhost:{}", ctx.config.port)).unwrap()
    });

    let cors = warp::cors()
        .allow_methods(vec!["GET", "PUT", "POST", "HEAD"])
        .allow_credentials(true)
        .allow_origin(domain.as_str())
        .build();

    let mut script_sources = vec![domain.as_str()];
    if ctx
        .config
        .dangerously_allow_script_eval_for_development_only
    {
        tracing::warn!("In development mode, script csp is currently broken");
        script_sources.push("'unsafe-eval'");
    }

    let csp = csp::Builder::new()
        .script_sources(script_sources)
        .worker_source(domain.as_str())
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

    // FIXME: don't panic when port already in use
    let addr = std::net::SocketAddr::new(ctx.config.ip_addr, ctx.config.port);
    let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, shutdown);

    tracing::info!("Listening on http://{}", addr);

    server.await;

    Ok(())
}

fn init_logging() {
    // FIXME: hack for default log level=info
    if let None = std::env::var_os("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init()
}

fn print_trace_and_exit(e: anyhow::Error) {
    let mut chain = e.chain();
    if let Some(head) = chain.next() {
        eprintln!("{}", head);
    }
    for cause in chain {
        eprintln!("Caused by: {}", cause);
    }

    std::process::exit(1);
}

fn main() {
    if let Some(cmd) = std::env::args_os().skip(1).next() {
        let cmd = cmd.to_string_lossy();
        let func = match cmd.as_ref() {
            "post-receive-hook" => crate::post_receive_hook::run,
            other => {
                eprintln!("Invalid subcommand `{}`, valid: `post-receive-hook`", other);
                std::process::exit(1);
            }
        };

        let mut rt = runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        if let Err(e) = rt.block_on(func()) {
            print_trace_and_exit(e);
        }
    } else {
        let mut rt = runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        if let Err(e) = rt.block_on(run()) {
            print_trace_and_exit(e);
        }
    }
}
