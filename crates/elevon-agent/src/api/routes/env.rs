use std::{collections::HashMap, sync::Arc};

use axum::{Json, Router, http::StatusCode, routing::post};
use elevon_fs::agent::add_app_env;
use elevon_http::error::AppError;
use serde::Deserialize;

use crate::{api::state::AppState, db::models::AuthKey};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(save))
}

#[derive(Deserialize)]
struct SaveDto {
    pub app_name: String,
    pub vars: HashMap<String, String>,
}

async fn save(_: AuthKey, Json(payload): Json<SaveDto>) -> Result<StatusCode, AppError> {
    for (key, value) in payload.vars {
        add_app_env(&payload.app_name, None, key, value)?;
    }

    Ok(StatusCode::OK)
}
