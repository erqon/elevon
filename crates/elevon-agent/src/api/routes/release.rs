use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use elevon_contracts::deploy::{AppReleasePayload, AppRole};
use elevon_http::error::AppError;

use crate::{
    api::{
        db::models::{App, AuthKey, Deployment, DeploymentStatus},
        state::AppState,
    },
    image::{pull_image, run_image},
    proxy::types::{AgentEvent, RouteConfig, RouteState},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(release))
}

async fn release(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppReleasePayload>,
) -> Result<StatusCode, AppError> {
    let mut db = state.agent_db.db.clone();

    for app in payload.apps {
        let db_app = App::get_or_create(&mut db, &app).await?;

        pull_image(&state.docker, &app).await?;
        let (deployment_id, container_id, port) =
            run_image(&state.docker, &db_app, &app, &mut db).await?;

        match (&app.role, &app.web_app) {
            (AppRole::Web, Some(web_app)) => {
                if let Some(mut current_deployment) =
                    Deployment::get_current_deployment(&mut db, &db_app.id).await?
                {
                    if let Some(container_id) = current_deployment.container_id.clone() {
                        toasty::update!(current_deployment {
                            status: DeploymentStatus::Drained
                        })
                        .exec(&mut db)
                        .await?;

                        let route_config = RouteConfig {
                            id: deployment_id.clone(),
                            name: app.name.clone(),
                            domain: web_app.domain.clone(),
                            port,
                            state: RouteState::Draining,
                            container_id,
                        };

                        let drain_stream = state.socket_client.connect().await?;
                        state
                            .socket_client
                            .send(
                                drain_stream,
                                AgentEvent::DrainRoute(route_config),
                            )
                            .await?;
                    }
                }

                let route_config = RouteConfig {
                    id: deployment_id,
                    name: app.name,
                    domain: web_app.domain.clone(),
                    port,
                    state: RouteState::Active,
                    container_id,
                };

                let upsert_stream = state.socket_client.connect().await?;
                state
                    .socket_client
                    .send(upsert_stream, AgentEvent::UpsertRoute(route_config))
                    .await?;
            }
            _ => {}
        }
    }

    Ok(StatusCode::OK)
}
