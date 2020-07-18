#[derive(serde::Deserialize)]
pub struct Login {
    pub name: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Register {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_check: String,
}
