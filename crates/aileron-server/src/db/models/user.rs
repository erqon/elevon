use uuid::Uuid;

#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto(uuid(v7))]
    pub id: Uuid,

    #[unique]
    pub email: String,

    pub first_name: Option<String>,
    pub last_name: Option<String>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[has_one]
    session: toasty::Deferred<Option<Session>>,
}

#[derive(Debug, toasty::Model)]
pub struct Session {
    #[key]
    #[auto(uuid(v7))]
    pub id: Uuid,

    #[unique]
    user_id: Uuid,

    token_hash: String,

    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub device_name: Option<String>,

    pub last_used_at: jiff::Timestamp,
    pub expires_at: jiff::Timestamp,
    #[auto]
    pub created_at: jiff::Timestamp,

    #[belongs_to(key = user_id, references = id)]
    user: toasty::Deferred<Option<User>>,
}

#[derive(toasty::Model)]
pub struct AccessKey {
    #[key]
    #[auto(uuid(v7))]
    pub id: Uuid,

    #[unique]
    user_id: Uuid,

    pub name: String,

    pub key_prefix: String,

    pub key_hash: String,

    pub last_used_at: Option<jiff::Timestamp>,
    pub revoked_at: Option<jiff::Timestamp>,
    #[auto]
    pub created_at: jiff::Timestamp,
}
