use std::{collections::HashMap, sync::Arc};

use axum::{Json, Router, http::StatusCode, routing::put};
use elevon_fs::agent::add_app_env;
use elevon_http::error::AppError;
use serde::Deserialize;

use crate::{api::state::AppState, db::models::AuthKey};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", put(set))
}

#[derive(Deserialize)]
struct AppEnvUpdate {
    pub app_name: String,
    pub vars: HashMap<String, String>,
}

#[derive(Deserialize)]
struct SetEnvDto {
    pub updates: Vec<AppEnvUpdate>,
}

async fn set(_: AuthKey, Json(payload): Json<SetEnvDto>) -> Result<StatusCode, AppError> {
    for update in payload.updates {
        for (key, value) in update.vars {
            add_app_env(&update.app_name, None, key, value)?;
        }
    }

    Ok(StatusCode::OK)
}
