use crate::{article::ArticleTitle, context::Wiki, index::SearchResult};
use askama::Template;

#[derive(Template)]
#[template(path = "edit.html")]
pub struct WikiEdit<'a> {
    pub title: &'a str,
    pub wiki: Wiki<'a>,
}

pub struct TitleSegment<'a> {
    pub relative_url: &'a str,
    pub segment_name: &'a str,
}

#[derive(Template)]
#[template(path = "wiki_page.html")]
pub struct WikiPage<'a> {
    pub title_segments: &'a [TitleSegment<'a>],
    pub title: &'a ArticleTitle,
    pub content: &'a str,
    pub wiki: Wiki<'a>,
}

#[derive(Template)]
#[template(path = "history.html")]
pub struct History<'a> {
    pub title: &'a str,
    pub history: &'a [crate::git::HistoryEntry],
    pub wiki: Wiki<'a>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login<'a> {
    pub wiki: Wiki<'a>,
    pub registration_enabled: bool,
    pub error: Option<&'a str>,
}

impl<'a> Login<'a> {
    pub fn new(
        ctx: &'a crate::context::Context,
        account: &'a Option<crate::user_storage::UserAccount>,
        error: Option<&'a str>,
    ) -> Self {
        Login {
            wiki: ctx.wiki(account),
            registration_enabled: ctx.registration_possible(),
            error,
        }
    }
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct Register<'a> {
    pub wiki: Wiki<'a>,
    pub error: Option<&'a str>,
}

impl<'a> Register<'a> {
    pub fn new(wiki: Wiki<'a>) -> Self {
        Self { wiki, error: None }
    }

    pub fn error(wiki: Wiki<'a>, error: &'a str) -> Self {
        Self {
            wiki,
            error: Some(error),
        }
    }
}

#[derive(Template)]
#[template(path = "search_results.html")]
pub struct SearchResults<'a> {
    pub wiki: Wiki<'a>,
    pub query: &'a str,
    pub results: &'a [SearchResult],
}

#[derive(Template)]
#[template(path = "register_refresh.html")]
pub struct RegisterRefresh<'a> {
    pub wiki: Wiki<'a>,
}

#[derive(Template)]
#[template(path = "add_article.html")]
pub struct AddArticle<'a> {
    pub wiki: Wiki<'a>,
}

#[derive(Template)]
#[template(path = "headline_start.html")]
pub struct HeadlineStart<'a> {
    pub strength: u32,
    pub headline: &'a str,
    pub id: &'a str,
}

#[derive(Template)]
#[template(path = "captioned_image.html")]
pub struct CaptionedImage<'a> {
    pub caption: &'a str,
    pub url: &'a str,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct Error<'a> {
    code: u16,
    msg: &'a str,
}

impl<'a> Error<'a> {
    pub fn internal_server() -> Self {
        Self {
            code: 500,
            msg: "Internal server error",
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: 404,
            msg: "Not found",
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: 400,
            msg: "Invalid request",
        }
    }

    pub fn not_implemented() -> Self {
        Self {
            code: 501,
            msg: "Not implemented",
        }
    }
}

#[derive(Template)]
#[template(path = "post-receive-hook.sh", escape = "none")]
pub struct PostReceiveHook {
    binary_path: String,
}

impl PostReceiveHook {
    pub fn new() -> Self {
        use std::os::unix::prelude::*;
        Self {
            binary_path: String::from_utf8(
                std::env::current_exe()
                    .expect("Can't find current executable")
                    .into_os_string()
                    .into_vec(),
            )
            .expect("Binary path is not utf-8"),
        }
    }
}

#[derive(Template)]
#[template(path = "wiki_root.html")]
pub struct Root<'a> {
    pub content: String,
    pub wiki: Wiki<'a>,
}
