use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use elevon_contracts::deploy::{ListQueryParams, StreamEvent};
use serde::Deserialize;

use crate::{
    api::{
        db::models::{App, AppTabled, AuthKey, Deployment, TabledView, app::DeploymentTabled},
        state::SharedApiState,
        stream::{StreamResponse, emit, spawn_streaming_task},
    },
    cli::key::{handle_key_delete, handle_key_list, handle_key_revoke},
};

pub fn router() -> Router<SharedApiState> {
    Router::new()
        .nest("/keys", KeyCommands::register_routes())
        .nest("/apps", AppCommands::register_routes())
        .nest("/deployments", DeploymentCommands::register_routes())
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

struct AppCommands;

impl AppCommands {
    async fn list(
        _: AuthKey,
        State(state): State<SharedApiState>,
        Query(query): Query<ListQueryParams>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            let mut db = state.db.get();

            let data: Vec<AppTabled> = App::all()
                .limit(query.limit)
                .offset(query.offset)
                .exec(&mut db)
                .await?
                .iter()
                .map(|r| r.to_tabled())
                .collect();

            let mut table = tabled::Table::new(data);
            table.with(tabled::settings::Style::modern());

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }
}

impl CommandsTrait for AppCommands {
    fn register_routes() -> Router<SharedApiState> {
        Router::new().route("/list", get(AppCommands::list))
    }
}

struct DeploymentCommands;

impl DeploymentCommands {
    async fn list(
        _: AuthKey,
        State(state): State<SharedApiState>,
        Query(query): Query<ListQueryParams>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            let mut db = state.db.get();

            let data: Vec<DeploymentTabled> = Deployment::all()
                .include(Deployment::fields().app())
                .limit(query.limit)
                .offset(query.offset)
                .exec(&mut db)
                .await?
                .into_iter()
                .map(|r| r.to_tabled())
                .collect();

            let mut table = tabled::Table::new(data);
            table.with(tabled::settings::Style::modern());

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }
}

impl CommandsTrait for DeploymentCommands {
    fn register_routes() -> Router<SharedApiState> {
        Router::new().route("/list", get(DeploymentCommands::list))
    }
}
