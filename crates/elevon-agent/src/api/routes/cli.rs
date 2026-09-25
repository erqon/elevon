use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use elevon_contracts::deploy::StreamEvent;
use serde::Deserialize;

use crate::api::{
    db::models::AuthKey,
    event::{handle_key_delete, handle_key_list, handle_key_revoke},
    state::SharedApiState,
    stream::{StreamResponse, emit, spawn_streaming_task},
};

pub fn router() -> Router<SharedApiState> {
    Router::new().nest("/key", KeyCommands::register_routes())
}

trait CommandsTrait {
    fn register_routes() -> Router<SharedApiState>;
}

struct KeyCommands;

impl KeyCommands {
    async fn create(
        _: AuthKey,
        State(state): State<SharedApiState>,
        Path(name): Path<String>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            emit(
                &tx,
                StreamEvent::log(format!("Creating auth key: {}", name)),
            )
            .await;

            let auth_key = AuthKey::create_key(&mut state.db.get(), &name).await;

            if auth_key.is_err() {
                emit(
                    &tx,
                    StreamEvent::log(format!(
                        "Failed: key with the name already exists: '{}'",
                        name,
                    )),
                )
                .await;

                return Ok(());
            }

            let auth_key = auth_key?;

            emit(
                &tx,
                StreamEvent::log(format!("Save your API Key: {}", auth_key)),
            )
            .await;

            Ok(())
        })
    }

    async fn list(_: AuthKey, State(state): State<SharedApiState>) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            let keys = handle_key_list(&mut state.db.get()).await?;
            let mut table = tabled::Table::new(keys);
            table.with(tabled::settings::Style::modern());

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }

    async fn revoke(
        _: AuthKey,
        State(state): State<SharedApiState>,
        Json(payload): Json<KeyListPayload>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            handle_key_revoke(payload.data, &mut state.db.get(), move |message| {
                let tx = tx.clone();
                async move {
                    emit(&tx, StreamEvent::log(message)).await;
                    Ok(())
                }
            })
            .await?;

            Ok(())
        })
    }

    async fn delete(
        _: AuthKey,
        State(state): State<SharedApiState>,
        Json(payload): Json<KeyListPayload>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            handle_key_delete(payload.data, &mut state.db.get(), move |message| {
                let tx = tx.clone();
                async move {
                    emit(&tx, StreamEvent::log(message)).await;
                    Ok(())
                }
            })
            .await?;

            Ok(())
        })
    }
}

#[derive(Deserialize)]
struct KeyListPayload {
    pub data: Vec<String>,
}

impl CommandsTrait for KeyCommands {
    fn register_routes() -> Router<SharedApiState> {
        Router::new()
            .route("/{name}", post(KeyCommands::create))
            .route("/list", get(KeyCommands::list))
            .route("/revoke", post(KeyCommands::revoke))
            .route("/delete", post(KeyCommands::delete))
    }
}
