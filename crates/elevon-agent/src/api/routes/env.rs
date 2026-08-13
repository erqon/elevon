use std::{collections::HashMap, sync::Arc};

use axum::{Json, Router, extract::State, http::StatusCode, routing::put};
use elevon_fs::agent::add_app_env;
use elevon_http::error::AppError;
use serde::Deserialize;

use crate::{
    api::{db::models::AuthKey, state::AppState},
    env::ElevonEnvKey,
};

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

async fn set(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetEnvDto>,
) -> Result<StatusCode, AppError> {
    for update in payload.updates {
        for (key, value) in update.vars {
            if let Ok(env_key) = key.parse::<ElevonEnvKey>() {
                let mut env = state.env.write().await;
                env.update_value(env_key, value)?;
            } else {
                add_app_env(&update.app_name, None, key.clone(), value.clone())?;
            }
        }
    }

    Ok(StatusCode::OK)
}
