#[derive(Debug, toasty::Model)]
pub struct AuthKey {
    #[key]
    #[auto]
    id: uuid::Uuid,

    api_key: String,

    #[auto]
    created_at: jiff::Timestamp,

    #[auto]
    updated_at: jiff::Timestamp,
}
