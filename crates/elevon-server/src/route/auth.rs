use elevon_auth::token::{hash, issue};
use axum::{
    Router,
    extract::{ConnectInfo, State},
    http::StatusCode,
    routing::{get, post},
};
use axum_extra::TypedHeader;
use headers::UserAgent;
use std::net::SocketAddr;

use crate::db::model::{AccessKey, Session};
use crate::service::auth::device_name_from_ua;
use crate::{
    app::error::{AppError, AppJson},
    service::auth::AuthUser,
};
use crate::{
    app::state::AppState,
    payload::auth::{AuthMe, LoginPayload, LoginResponse},
};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/logout", post(logout))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    AppJson(payload): AppJson<LoginPayload>,
) -> Result<AppJson<LoginResponse>, AppError> {
    let mut db_pool = state.db.clone();

    let hashed_key = hash(&payload.key);
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

    let session_token = issue();
    let now = jiff::Timestamp::now();
    let expires_at = now + jiff::Span::new().hours(24);

    toasty::create!(Session {
        user_id: access_key.user_id,
        user_agent: user_agent.as_str(),
        ip_address: addr.ip().to_string(),
        device_name: device_name_from_ua(user_agent.as_str()),
        token_hash: session_token.hash,
        last_used_at: now,
        expires_at
    })
    .exec(&mut db_pool)
    .await?;

    Ok(AppJson(LoginResponse {
        session_token: session_token.raw,
    }))
}

async fn me(AuthUser { user, session_id }: AuthUser) -> Result<AppJson<AuthMe>, AppError> {
    Ok(AppJson(AuthMe::from_user(user, session_id)))
}

async fn logout(
    AuthUser {
        user: _,
        session_id,
    }: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let mut db_pool = state.db.clone();

    Session::delete_by_id(&mut db_pool, session_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
