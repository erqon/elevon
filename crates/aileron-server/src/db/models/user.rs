use uuid::Uuid;

use crate::db::models::auth::Session;

#[derive(Clone, Debug, toasty::Model)]
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
    session: toasty::Deferred<Vec<Session>>,
}
