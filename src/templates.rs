use askama::Template;

#[derive(Template)]
#[template(path = "wiki_page.html")]
pub struct WikiPage<'a> {
    pub title: &'a str,
    pub content: &'a str,
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
}
