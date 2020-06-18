use crate::data::Wiki;
use askama::Template;

#[derive(Template)]
#[template(path = "edit.html")]
pub struct WikiEdit<'a> {
    pub wiki: Wiki<'a>,
}

#[derive(Template)]
#[template(path = "wiki_page.html")]
pub struct WikiPage<'a> {
    pub title: &'a str,
    pub content: &'a str,
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
    pub fn new(data: &'a crate::data::Data, error: Option<&'a str>) -> Self {
        Login {
            wiki: data.wiki(),
            registration_enabled: data.registration_possible(),
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
    pub results: &'a [crate::article::ArticleTitle],
}

#[derive(Template)]
#[template(path = "register_refresh.html")]
pub struct RegisterRefresh<'a> {
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
