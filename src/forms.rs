#[derive(serde::Deserialize)]
pub struct Login {
    pub name: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct Register {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct NewArticle {
    pub markdown: String,
}
