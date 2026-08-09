use std::sync::Arc;

use axum::{Router, http::StatusCode, routing::post};
use elevon_http::error::AppError;

use crate::api::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(save))
}

async fn save() -> Result<StatusCode, AppError> {
    Ok(StatusCode::OK)
}
