use axum::{Router, extract::State, routing::post};
use serde::Deserialize;

use crate::app::error::{AppError, AppJson};
use crate::app::state::AppState;
use crate::db::models::user::User;
pub fn auth_router() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

async fn login(
    State(mut state): State<AppState>,
    AppJson(payload): AppJson<Login>,
) -> Result<(), AppError> {
    let user = match User::get_by_email(&mut state.db, payload.email).await {
        Ok(user) => user,
        Err(err) if err.is_record_not_found() => return Err(AppError::Unauthorized),
        Err(err) => return Err(AppError::Db(err)),
    };

    let password_is_valid = user
        .verify_password(&payload.password)
        .map_err(AppError::from)?;

    if !password_is_valid {
        return Err(AppError::Unauthorized);
    }

    Ok(())
}
