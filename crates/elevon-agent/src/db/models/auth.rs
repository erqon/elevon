use std::sync::Arc;

use axum::extract::FromRequestParts;
use elevon_http::{auth::get_auth_token, error::AppError, token::hash};

use crate::api::state::AppState;

#[derive(Debug, toasty::Model)]
pub struct AuthKey {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

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

impl FromRequestParts<Arc<AppState>> for AuthKey {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = get_auth_token(&parts.headers)?;

        let db = state.db.clone();
        let auth_key = check_auth_key(db, token).await?;

        Ok(auth_key)
    }
}

async fn check_auth_key(mut db: toasty::Db, auth_key: String) -> anyhow::Result<AuthKey, AppError> {
    let hashed_key = hash(&auth_key);

    let mut auth_key = AuthKey::get_by_key_hash(&mut db, hashed_key)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if auth_key.enabled && auth_key.expires_at < jiff::Timestamp::now() {
        return Err(AppError::Unauthorized);
    }

    toasty::update!(auth_key {
        last_used_at: Some(jiff::Timestamp::now())
    })
    .exec(&mut db)
    .await?;

    Ok(auth_key)
}
