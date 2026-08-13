use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use elevon_http::error::AppError;
use serde::Deserialize;

use crate::{
    api::{db::models::AuthKey, dto::AppDeployData, state::AppState},
    image::{pull_image, run_image},
    proxy::types::{AgentEvent, RouteConfig},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(deploy))
}

#[derive(Debug, Deserialize)]
struct DeployDto {
    pub apps: Vec<AppDeployData>,
}

async fn deploy(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployDto>,
) -> Result<StatusCode, AppError> {
    let mut db = state.db.clone();

    let mut success_count: Vec<String> = Vec::with_capacity(payload.apps.len());
    let env_snapshot = {
        let guard = state.env.read().await;
        guard.clone()
    };

    for app in payload.apps {
        pull_image(&env_snapshot, &app).await?;
        let (app, port) = run_image(&env_snapshot, &app, &mut db).await?;

        let stream = state.socket_client.connect().await?;

        let route_config = RouteConfig {
            id: app.id.to_string(),
            domain: app.domain.to_string(),
            port: port,
        };

        state
            .socket_client
            .send(stream, AgentEvent::UpsertRoute(route_config))
            .await?;

        success_count.push(app.name);
    }

    Ok(StatusCode::OK)
}
