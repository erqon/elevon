use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::post};
use elevon_contracts::deploy::{AppDeployPayload, AppRollbackPayload, StreamEvent};

use crate::{
    api::{
        db::models::AuthKey,
        state::AppState,
        stream::{StreamResponse, StreamSender, create_stream_channel, emit, stream_response},
    },
    image::{deploy_apps, rollback_apps},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(deploy))
        .route("/rollback", post(rollback))
}

fn spawn_streaming_task<F, Fut>(op: F) -> StreamResponse
where
    F: FnOnce(StreamSender) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (tx, rx) = create_stream_channel();
    let tx_for_task = tx.clone();

    tokio::spawn(async move {
        if let Err(err) = op(tx_for_task.clone()).await {
            emit(
                &tx_for_task,
                StreamEvent::Error {
                    message: err.to_string(),
                },
            )
            .await;
        }

        emit(&tx_for_task, StreamEvent::Done).await;
    });

    stream_response(rx)
}

async fn deploy(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppDeployPayload>,
) -> StreamResponse {
    spawn_streaming_task(move |tx| async move { deploy_apps(&tx, state, payload.apps).await })
}

async fn rollback(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppRollbackPayload>,
) -> StreamResponse {
    spawn_streaming_task(move |tx| async move { rollback_apps(&tx, state, payload.apps).await })
}
