use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header},
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    app::{error::AppError, state::AppState},
    db::models::{Session, User},
};

pub fn create_session_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);

    let raw = BASE64_URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_raw_token(&raw);

    (raw, hash)
}

pub fn hash_raw_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[derive(Clone, Debug)]
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

    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').map(str::trim).find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == "session_token").then(|| value.to_string())
    })
}

async fn check_user_session(
    db: &mut toasty::Db,
    session_token: String,
) -> Result<AuthUser, AppError> {
    let hashed_token = hash_raw_token(&session_token);

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
        return Err(AppError::Unauthorized);
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
