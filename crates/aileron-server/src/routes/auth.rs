use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Router, extract::State, routing::post};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::app::error::{AppError, AppJson};
use crate::app::state::AppState;
use crate::db::models::user::User;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

async fn login(
    State(mut state): State<AppState>,
    jar: CookieJar,
    AppJson(payload): AppJson<Login>,
) -> Result<(CookieJar, StatusCode), AppError> {
    let user = match User::get_by_email(&mut state.db, payload.email).await {
        Ok(user) => user,
        Err(err) if err.is_record_not_found() => return Err(AppError::Unauthorized),
        Err(err) => return Err(AppError::Db(err)),
    };

    if !user.verify_password(&payload.password)? {
        return Err(AppError::Unauthorized);
    }

    let token = state
        .tokens
        .create_access_token(user.id)
        .map_err(AppError::from)?;

    let cookie = Cookie::build(("access_token", token))
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::hours(1))
        .build();

    Ok((jar.add(cookie), StatusCode::NO_CONTENT))
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("access_token", ""))
        .http_only(true)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();

    (jar.add(cookie), StatusCode::NO_CONTENT)
}
