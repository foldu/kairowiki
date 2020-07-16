use crate::serde::Oid;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Commit {
    Merged {
        merged: String,
        oid: Oid,
        rev: Oid,
    },

    Conflict {
        ancestor: Option<String>,
        ours: String,
        theirs: String,
        oid: Oid,
        rev: Oid,
    },

    NoConflict,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleInfo {
    pub markdown: String,
    pub oid: Option<Oid>,
    pub rev: Oid,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMarkdown {
    pub markdown: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSubmit {
    pub commit_msg: String,
    pub markdown: String,
    pub oid: Option<Oid>,
    pub rev: Oid,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedMarkdown {
    pub rendered: String,
}

