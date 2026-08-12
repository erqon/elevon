use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use elevon_http::error::AppError;
use serde::Deserialize;

use crate::{
    api::{dto::AppDeployData, state::AppState},
    db::models::AuthKey,
    image::{pull_image, run_image},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(deploy))
}

#[derive(Debug, Deserialize)]
struct DeployDto {
    pub apps: Vec<AppDeployData>,
}

// During deployments data should be saved in db about the apps, then IDs should be
// generated so each app would have a unique id, each app should have the port in the
// [port:port+1] range, so each deployment deploys on empty port. There should also be
// some info about the backups that will be kept on the agent's machine, in deploy's config
// so each deployment would just delete previous images.

async fn deploy(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployDto>,
) -> Result<StatusCode, AppError> {
    let mut success_count: Vec<String> = Vec::with_capacity(payload.apps.len());
    let env_snapshot = {
        let guard = state.env.read().await;
        guard.clone()
    };

    for app in payload.apps {
        pull_image(&env_snapshot, &app).await?;
        run_image(&env_snapshot, &app).await?;

        success_count.push(app.name);
    }

    Ok(StatusCode::OK)
}
