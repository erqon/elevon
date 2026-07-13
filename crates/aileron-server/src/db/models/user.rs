#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto(uuid(v7))]
    pub id: uuid::Uuid,

    #[unique]
    pub email: String,

    pub first_name: Option<String>,

    pub last_name: Option<String>,

    password: String,

    #[auto]
    pub created_at: jiff::Timestamp,
}

impl User {
    pub fn verify_password(&self, password: &str) -> anyhow::Result<bool> {
        crate::services::auth::verify_password(password, &self.password)
    }
}
