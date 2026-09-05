use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, routing::get};
use elevon_http::error::AppError;

use crate::{
    api::{middleware::local_only, state::AppState},
    proxy::types::DeployAppData,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/containers", get(get_running_containers))
        .route_layer(axum::middleware::from_fn(local_only))
}

async fn get_running_containers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeployAppData>>, AppError> {
    let containers = state.get_running_route_containers().await?;
    Ok(Json(containers))
}
