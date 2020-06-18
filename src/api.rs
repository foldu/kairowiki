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
