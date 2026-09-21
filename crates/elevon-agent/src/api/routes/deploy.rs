use std::fs::{OpenOptions, TryLockError};

use axum::{Json, Router, extract::State, routing::post};
use elevon_contracts::deploy::{AppDeployPayload, AppRollbackPayload, StreamEvent};
use elevon_fs::agent::AgentPath;

use crate::{
    api::{
        db::models::AuthKey,
        state::SharedApiState,
        stream::{StreamResponse, StreamSender, create_stream_channel, emit, stream_response},
    },
    image::{deploy_apps, rollback_apps},
};

pub fn router() -> Router<SharedApiState> {
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

fn lock_action() -> anyhow::Result<std::fs::File> {
    let lock_path = AgentPath::DeployLock.resolve();
    let lock_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(lock_path)?;

    match lock_file.try_lock() {
        Ok(()) => Ok(lock_file),
        Err(TryLockError::WouldBlock) => {
            anyhow::bail!("another deployment is already running")
        }
        Err(TryLockError::Error(err)) => Err(err.into()),
    }
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
