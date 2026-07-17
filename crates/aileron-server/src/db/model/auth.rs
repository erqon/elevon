use uuid::Uuid;

use crate::db::model::user::User;

#[derive(toasty::Model)]
pub struct Session {
    #[key]
    #[auto(uuid(v7))]
    pub id: Uuid,

    #[index]
    pub user_id: Uuid,

    pub token_hash: String,

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
    pub user_id: Uuid,

    pub name: String,

    pub key_prefix: String,

    #[unique]
    pub key_hash: String,

    pub last_used_at: Option<jiff::Timestamp>,
    pub revoked_at: Option<jiff::Timestamp>,
    #[auto]
    pub created_at: jiff::Timestamp,
}

#[derive(toasty::Embed)]
pub enum LoginEventOucome {
    Success,
    InvalidKey,
}

#[derive(toasty::Model)]
pub struct LoginEvent {
    #[key]
    #[auto(uuid(v7))]
    pub id: Uuid,

    #[index]
    pub user_id: Uuid,

    #[index]
    pub access_key_id: Uuid,

    pub ip_address: String,
    pub user_agent: String,
    pub outcome: LoginEventOucome,

    #[belongs_to(key = user_id, references = id)]
    pub user: toasty::Deferred<User>,
}
