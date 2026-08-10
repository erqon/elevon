pub mod env;

use std::sync::Arc;

use axum::{Router, http::StatusCode, middleware::from_fn, routing::get};
use elevon_http::error::AppError;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::api::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    Router::new()
        .route("/health", get(health))
        .nest("/env", crate::api::routes::env::router())
        .with_state(state)
        .layer(from_fn(elevon_http::error::log_app_errors))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
}

async fn health() -> Result<StatusCode, AppError> {
    Ok(StatusCode::OK)
}
