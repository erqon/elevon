use axum::{
    Json, Router,
    extract::{Query, State},
    middleware,
    response::Result,
    routing::{get, post},
};
use elevon_contracts::{
    cli::{ListQueryParams, app::RemoveAppCommandArgs},
    deploy::StreamEvent,
};
use elevon_http::error::AppError;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tabled::settings::panel::Footer;

use crate::{
    api::{
        db::models::{
            App, AppTabled, AuthKey, AuthKeyTabled, Deployment, TabledView,
            deployment::DeploymentTabled,
        },
        middleware::{auth_middleware, require_unix_socket},
        state::SharedApiState,
        stream::{StreamResponse, emit, spawn_streaming_task},
    },
    cli::{
        app::handle_app_stop,
        key::{handle_key_delete, handle_key_revoke},
    },
};

pub fn router(state: SharedApiState) -> Router<SharedApiState> {
    Router::new()
        .nest("/keys", KeyCommands::register_routes(state.clone()))
        .nest("/apps", AppCommands::register_routes(state.clone()))
        .nest(
            "/deployments",
            DeploymentCommands::register_routes(state.clone()),
        )
}

trait CommandsTrait {
    fn create_router() -> Router<SharedApiState>;

    fn register_routes(state: SharedApiState) -> Router<SharedApiState> {
        let base_router = Self::create_router();

        let unix_router = base_router
            .clone()
            .route_layer(middleware::from_fn(require_unix_socket));

        let auth_router = base_router.route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

        Router::new().nest("/local", unix_router).merge(auth_router)
    }
}

struct KeyCommands;

impl KeyCommands {
    async fn create(
        State(state): State<SharedApiState>,
        Json(payload): Json<Value>,
    ) -> Result<StreamResponse, AppError> {
        let Some(name) = payload
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(AppError::client(
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Missing or invalid 'name' field",
            ));
        };

        Ok(spawn_streaming_task(move |tx| async move {
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
        }))
    }

    async fn list(
        State(state): State<SharedApiState>,
        Query(query): Query<ListQueryParams>,
    ) -> StreamResponse {
        let mut db = state.db.get();

        spawn_streaming_task(move |tx| async move {
            let count = AuthKey::all().count().exec(&mut db).await?;
            let (limit, total_pages, offset) = query.get_limits(count);

            let data: Vec<AuthKeyTabled> = AuthKey::all()
                .limit(limit)
                .offset(offset)
                .exec(&mut db)
                .await?
                .iter()
                .map(|r| r.to_tabled())
                .collect();

            let mut table = tabled::Table::new(data);
            table.with(tabled::settings::Style::modern());

            let footer_text = format!("Page {}/{}", query.page, total_pages);
            table.with(Footer::new(footer_text));

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }

    async fn revoke(
        State(state): State<SharedApiState>,
        Json(payload): Json<KeyListPayload>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            handle_key_revoke(payload.data, &mut state.db.get(), &tx).await
        })
    }

    async fn delete(
        State(state): State<SharedApiState>,
        Json(payload): Json<KeyListPayload>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            handle_key_delete(payload.data, &mut state.db.get(), &tx).await
        })
    }
}

impl CommandsTrait for KeyCommands {
    fn create_router() -> Router<SharedApiState> {
        Router::new()
            .route("/", post(KeyCommands::create))
            .route("/list", get(KeyCommands::list))
            .route("/revoke", post(KeyCommands::revoke))
            .route("/delete", post(KeyCommands::delete))
    }
}

#[derive(Serialize, Deserialize)]
pub struct KeyListPayload {
    pub data: Vec<String>,
}

struct AppCommands;

impl AppCommands {
    async fn list(
        State(state): State<SharedApiState>,
        Query(query): Query<ListQueryParams>,
    ) -> StreamResponse {
        let mut db = state.db.get();

        spawn_streaming_task(move |tx| async move {
            let count = App::all().count().exec(&mut db).await?;
            let (limit, total_pages, offset) = query.get_limits(count);

            let data: Vec<AppTabled> = App::all()
                .limit(limit)
                .offset(offset)
                .exec(&mut db)
                .await?
                .iter()
                .map(|r| r.to_tabled())
                .collect();

            let mut table = tabled::Table::new(data);
            table.with(tabled::settings::Style::modern());

            let footer_text = format!("Page {}/{}", query.page, total_pages);
            table.with(Footer::new(footer_text));

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }

    async fn stop(
        State(state): State<SharedApiState>,
        Json(payload): Json<RemoveAppCommandArgs>,
    ) -> StreamResponse {
        spawn_streaming_task(move |tx| async move {
            handle_app_stop(payload.apps, payload.force, state, &tx).await
        })
    }
}

impl CommandsTrait for AppCommands {
    fn create_router() -> Router<SharedApiState> {
        Router::new()
            .route("/list", get(AppCommands::list))
            .route("/stop", post(AppCommands::stop))
    }
}

struct DeploymentCommands;

impl DeploymentCommands {
    async fn list(
        State(state): State<SharedApiState>,
        Query(query): Query<ListQueryParams>,
    ) -> StreamResponse {
        let mut db = state.db.get();

        spawn_streaming_task(move |tx| async move {
            let count = Deployment::all().count().exec(&mut db).await?;
            let (limit, total_pages, offset) = query.get_limits(count);

            let data: Vec<DeploymentTabled> = Deployment::all()
                .include(Deployment::fields().app())
                .limit(limit)
                .offset(offset)
                .exec(&mut db)
                .await?
                .into_iter()
                .map(|r| r.to_tabled())
                .collect();

            let mut table = tabled::Table::new(data);
            table.with(tabled::settings::Style::modern());

            let footer_text = format!("Page {}/{}", query.page, total_pages);
            table.with(Footer::new(footer_text));

            emit(&tx, StreamEvent::log(format!("{}", table))).await;

            Ok(())
        })
    }
}

impl CommandsTrait for DeploymentCommands {
    fn create_router() -> Router<SharedApiState> {
        Router::new().route("/list", get(DeploymentCommands::list))
    }
}
