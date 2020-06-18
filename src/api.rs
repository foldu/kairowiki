use serde::{de::Deserializer, Deserialize};

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
    pub oid: Option<HexEncode<git2::Oid>>,
}

pub struct HexEncode<T>(pub T);

impl<T> serde::Serialize for HexEncode<T>
where
    T: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = hex::encode(self.0.as_ref());
        serializer.serialize_str(&hex)
    }
}

fn deserialize_oid<'de, D>(deserializer: D) -> Result<Option<git2::Oid>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => git2::Oid::from_str(&s).map(Some).map_err(D::Error::custom),
        None => Ok(None),
    }
}

#[derive(serde::Deserialize)]
pub struct PreviewMarkdown {
    pub markdown: String,
}

#[derive(serde::Deserialize)]
pub struct EditSubmit {
    pub markdown: String,
    #[serde(deserialize_with = "deserialize_oid")]
    pub oid: Option<git2::Oid>,
}

#[derive(serde::Serialize)]
pub struct RenderedMarkdown {
    pub rendered: String,
}
