use axum::{
    Router, extract::ConnectInfo, extract::State, http::StatusCode, response::IntoResponse,
    routing::post,
};
use axum_extra::{
    TypedHeader,
    extract::cookie::{Cookie, CookieJar, SameSite},
};
use headers::UserAgent;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::app::error::{AppError, AppJson};
use crate::app::state::AppState;
use crate::db::models::{AccessKey, Session};
use crate::services::auth::{create_session_token, device_name_from_ua, hash_raw_token};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
}

#[derive(Deserialize)]
struct Login {
    key: String,
}

async fn login(
    State(mut state): State<AppState>,
    jar: CookieJar,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AppJson(payload): AppJson<Login>,
) -> Result<(CookieJar, StatusCode), AppError> {
    let hashed_key = hash_raw_token(&payload.key);
    let Some(access_key) = AccessKey::filter(AccessKey::fields().key_hash().eq(hashed_key))
        .first()
        .exec(&mut state.db)
        .await?
    else {
        return Err(AppError::Unauthorized);
    };

    let (raw, hash) = create_session_token();
    let now = jiff::Timestamp::now();
    let expires_at = now + jiff::Span::new().hours(24);

    toasty::create!(Session {
        user_id: access_key.user_id,
        user_agent: user_agent.as_str(),
        ip_address: addr.ip().to_string(),
        device_name: device_name_from_ua(user_agent.as_str()),
        token_hash: hash,
        last_used_at: now,
        expires_at
    })
    .exec(&mut state.db)
    .await?;

    let session_token = Cookie::build(("session_token", raw))
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::hours(1))
        .build();

    Ok((jar.add(session_token), StatusCode::NO_CONTENT))
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("session_token", ""))
        .http_only(true)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();

    (jar.add(cookie), StatusCode::NO_CONTENT)
}
