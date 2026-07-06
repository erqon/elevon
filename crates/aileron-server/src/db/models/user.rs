#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto(uuid(v7))]
    id: uuid::Uuid,

    #[unique]
    email: String,

    first_name: Option<String>,

    last_name: Option<String>,

    password: String,

    #[auto]
    created_at: jiff::Timestamp,
}

impl User {
    pub fn verify_password(&self, password: &str) -> anyhow::Result<bool> {
        crate::services::auth::verify_password(password, &self.password)
    }
}
