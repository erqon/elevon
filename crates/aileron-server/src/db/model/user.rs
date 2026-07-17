use uuid::Uuid;

use crate::db::model::auth::{LoginEvent, Session};

#[derive(toasty::Model)]
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

    #[has_many]
    pub sessions: toasty::Deferred<Vec<Session>>,

    #[has_many]
    pub login_events: toasty::Deferred<Vec<LoginEvent>>,
}
