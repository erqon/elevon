use axum::{
    Router,
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::{
    TypedHeader,
    extract::cookie::{Cookie, CookieJar, SameSite},
};
use headers::UserAgent;
use std::net::SocketAddr;

use crate::app::state::AppState;
use crate::db::model::{AccessKey, Session};
use crate::payload::auth::LoginPayload;
use crate::service::auth::{create_session_token, device_name_from_ua, hash_raw_token};
use crate::{
    app::error::{AppError, AppJson},
    service::auth::AuthUser,
};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    jar: CookieJar,
    AppJson(payload): AppJson<LoginPayload>,
) -> Result<(CookieJar, StatusCode), AppError> {
    let mut db_pool = state.db.clone();

    let hashed_key = hash_raw_token(&payload.key);
    let Some(access_key) = AccessKey::filter(AccessKey::fields().key_hash().eq(hashed_key))
        .first()
        .exec(&mut db_pool)
        .await?
    else {
        return Err(AppError::client(
            StatusCode::UNAUTHORIZED,
            "invalid_access_key",
            "Invalid access key",
        ));
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
    .exec(&mut db_pool)
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

async fn me(
    AuthUser { user, session_id }: AuthUser,
) -> anyhow::Result<AppJson<serde_json::Value>, AppError> {
    Ok(AppJson(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "created_at": user.created_at.to_string(),
        "session_id": session_id,
    })))
}
