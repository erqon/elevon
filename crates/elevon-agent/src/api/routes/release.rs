use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::post};
use elevon_contracts::deploy::{AppReleasePayload, StreamEvent};

use crate::{
    api::{
        db::models::AuthKey,
        state::AppState,
        stream::{StreamResponse, create_stream_channel, emit, stream_response},
    },
    image::deploy_apps,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(release))
}

async fn release(
    _: AuthKey,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppReleasePayload>,
) -> StreamResponse {
    let (tx, rx) = create_stream_channel();

    let tx_for_task = tx.clone();
    let payload_for_task = payload;

    tokio::spawn(async move {
        if let Err(err) = deploy_apps(&tx, state, payload_for_task.apps).await {
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
