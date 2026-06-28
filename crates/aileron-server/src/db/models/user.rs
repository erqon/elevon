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

    #[has_one]
    role: toasty::Deferred<Option<Role>>,
}

#[derive(Debug, toasty::Model)]
pub struct Role {
    #[key]
    #[auto(uuid(v7))]
    id: uuid::Uuid,

    #[index]
    user_id: uuid::Uuid,

    name: String,

    #[belongs_to(key = user_id, references = id)]
    user: toasty::Deferred<User>,
}
