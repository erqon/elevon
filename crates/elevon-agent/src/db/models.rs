#[derive(Debug, toasty::Model)]
pub struct AuthKey {
    #[key]
    #[auto]
    id: uuid::Uuid,

    pub name: String,

    key_hash: String,

    pub enabled: bool,

    pub last_used_at: Option<jiff::Timestamp>,

    pub expires_at: jiff::Timestamp,

    pub revoked_at: Option<jiff::Timestamp>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}
