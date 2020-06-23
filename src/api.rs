#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Commit {
    Diff { a: String, b: String },

    Ok,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ArticleInfo {
    pub markdown: String,
    pub oid: Option<crate::serde::HexEncode<git2::Oid>>,
}

#[derive(serde::Deserialize)]
pub struct PreviewMarkdown {
    pub markdown: String,
}

#[derive(serde::Deserialize)]
pub struct EditSubmit {
    pub markdown: String,
    #[serde(deserialize_with = "crate::serde::deserialize_oid")]
    pub oid: Option<git2::Oid>,
}

#[derive(serde::Serialize)]
pub struct RenderedMarkdown {
    pub rendered: String,
}
