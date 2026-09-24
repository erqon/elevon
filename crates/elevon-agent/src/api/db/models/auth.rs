use anyhow::{Context, Result};
use axum::extract::FromRequestParts;
use elevon_http::{auth::get_auth_token, error::AppError, token::hash};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::api::state::SharedApiState;

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

impl AuthKey {
    pub async fn create_key(db: &mut toasty::Db, name: &str) -> Result<String> {
        let api_key = elevon_http::token::opaque();
        let hashed_api_key = elevon_http::token::hash(&api_key);

        let _ = AuthKey::get_by_name(db, name)
            .await
            .context("name already used")?;

        let now = jiff::Timestamp::now();
        let expires_at = now.checked_add(jiff::Span::new().hours(30 * 24))?;

        toasty::create!(AuthKey {
            name: name.to_string(),
            key_hash: hashed_api_key,
            enabled: true,
            expires_at
        })
        .exec(db)
        .await?;

        Ok(api_key)
    }
}

impl FromRequestParts<SharedApiState> for AuthKey {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &SharedApiState,
    ) -> Result<Self, Self::Rejection> {
        let token = get_auth_token(&parts.headers)?;

        let db = state.db.get();
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

#[derive(Tabled, Serialize, Deserialize)]
pub struct AuthKeyTableRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub expires: String,
    pub last_used: String,
    pub revoked: String,
}
