#[derive(Debug, toasty::Model)]
pub struct AuthKey {
    #[key]
    #[auto]
    id: uuid::Uuid,

    #[unique]
    pub name: String,

    #[unique]
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

impl std::fmt::Display for AuthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name={} enabled={} expires_at={} last_used_at={:?} revoked_at={:?}",
            self.name, self.enabled, self.expires_at, self.last_used_at, self.revoked_at
        )
    }
}
