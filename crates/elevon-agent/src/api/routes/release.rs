use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use elevon_contracts::deploy::{AppReleasePayload, AppRole};
use elevon_http::error::AppError;

use crate::{
    api::{db::models::AuthKey, state::AppState},
    image::{pull_image, run_image},
    proxy::types::{AgentEvent, RouteConfig},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(release))
}

async fn release(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppReleasePayload>,
) -> Result<StatusCode, AppError> {
    let mut db = state.db.clone();

    for app in payload.apps {
        pull_image(&app).await?;
        let (deployment_id, port) = run_image(&app, &mut db).await?;

        match (&app.role, &app.web_app) {
            (AppRole::Web, Some(web_app)) => {
                let stream = state.socket_client.connect().await?;

                let route_config = RouteConfig {
                    id: deployment_id,
                    domain: web_app.domain.clone(),
                    port,
                };

                state
                    .socket_client
                    .send(stream, AgentEvent::UpsertRoute(route_config))
                    .await?;
            }
            _ => {}
        }
    }

    Ok(StatusCode::OK)
}
