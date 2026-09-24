use axum::{Json, Router, extract::State, routing::post};
use elevon_contracts::deploy::{AppDeployPayload, AppRollbackPayload};

use crate::{
    api::{
        db::models::AuthKey,
        state::SharedApiState,
        stream::{StreamResponse, lock_action, spawn_streaming_task},
    },
    image::{deploy_apps, rollback_apps},
};

pub fn router() -> Router<SharedApiState> {
    Router::new()
        .route("/", post(deploy))
        .route("/rollback", post(rollback))
}

async fn deploy(
    _: AuthKey,
    State(state): State<SharedApiState>,
    Json(payload): Json<AppDeployPayload>,
) -> StreamResponse {
    spawn_streaming_task(move |tx| async move {
        let _lock_file = lock_action()?;
        deploy_apps(&tx, state, payload).await
    })
}

async fn rollback(
    _: AuthKey,
    State(state): State<SharedApiState>,
    Json(payload): Json<AppRollbackPayload>,
) -> StreamResponse {
    spawn_streaming_task(move |tx| async move {
        let _lock_file = lock_action()?;
        rollback_apps(&tx, state, payload).await
    })
}
