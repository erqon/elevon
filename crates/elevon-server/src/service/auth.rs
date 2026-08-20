use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header},
};
use elevon_http::token::{access_key, hash};
use uuid::Uuid;

use crate::{
    app::{error::AppError, state::AppState},
    db::model::{AccessKey, Session, User},
};

pub async fn create_access_key(
    mut db: toasty::Db,
    user_id: Uuid,
    name: impl Into<String>,
) -> Result<(String, AccessKey)> {
    let key = access_key();
    let hash = hash(&key);

    let access_key = toasty::create!(AccessKey {
        user_id,
        name: name.into(),
        key_prefix: key[..16].to_string(), // elevon_ -> 8 length + ak_...
        key_hash: hash,
    })
    .exec(&mut db)
    .await?;

    Ok((key, access_key))
}

pub struct AuthUser {
    pub user: User,
    pub session_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> std::prelude::v1::Result<Self, Self::Rejection> {
        let token = extract_token_from_headers(&parts.headers).ok_or(AppError::Unauthorized)?;

        let mut db = state.db.clone();
        let auth_user = check_user_session(&mut db, token).await?;

        Ok(auth_user)
    }
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        let value = value.to_str().ok()?;
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    None
}

async fn check_user_session(
    db: &mut toasty::Db,
    session_token: String,
) -> Result<AuthUser, AppError> {
    let hashed_token = hash(&session_token);

    let session = Session::filter(Session::fields().token_hash().eq(hashed_token))
        .first()
        .exec(db)
        .await?;

    if let Some(session) = session {
        if session.expires_at < jiff::Timestamp::now() {
            session.delete().exec(db).await?;
            return Err(AppError::Unauthorized);
        }

        let user = User::get_by_id(db, session.user_id).await?;

        Ok(AuthUser {
            user,
            session_id: session.id,
        })
    } else {
        Err(AppError::Unauthorized)
    }
}

pub fn device_name_from_ua(ua: &str) -> String {
    let platform = ua
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(inside, _)| {
            inside
                .split(';')
                .map(str::trim)
                .find(|part| {
                    part.starts_with("Linux")
                        || part.starts_with("Windows")
                        || part.starts_with("Macintosh")
                        || part.starts_with("Android")
                        || part.starts_with("iPhone")
                        || part.starts_with("iPad")
                })
                .map(|part| match part {
                    "Macintosh" => "macOS",
                    other => other,
                })
                .unwrap_or("Unknown")
        })
        .unwrap_or("Unknown");

    let browser = if ua.contains("Edg/") {
        "Edge"
    } else if ua.contains("Chrome/") && !ua.contains("Edg/") {
        "Chrome"
    } else if ua.contains("Firefox/") {
        "Firefox"
    } else if ua.contains("Safari/") && !ua.contains("Chrome/") {
        "Safari"
    } else {
        "Unknown"
    };

    format!("{platform}, {browser}")
}
