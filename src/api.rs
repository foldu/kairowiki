use crate::serde::Oid;

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Commit {
    Merged { merged: String, oid: Oid, rev: Oid },

    NoConflict,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ArticleInfo {
    pub markdown: String,
    pub oid: Option<Oid>,
    pub rev: Oid,
}

#[derive(serde::Deserialize)]
pub struct PreviewMarkdown {
    pub markdown: String,
}

#[derive(serde::Deserialize)]
pub struct EditSubmit {
    pub commit_msg: String,
    pub markdown: String,
    pub oid: Option<Oid>,
    pub rev: Oid,
}

#[derive(serde::Serialize)]
pub struct RenderedMarkdown {
    pub rendered: String,
}
