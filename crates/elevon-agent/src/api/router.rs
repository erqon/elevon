use std::sync::Arc;

use axum::{Router, extract::State, http::StatusCode, middleware::from_fn, routing::post};
use elevon_http::error::AppError;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::api::state::AppState;

pub fn create_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/deploy", post(deploy))
        .with_state(state)
        .layer(from_fn(elevon_http::error::log_app_errors))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
}

async fn deploy(State(state): State<Arc<AppState>>) -> Result<StatusCode, AppError> {
    let stream = state.socket_client.connect().await?;

    Ok(StatusCode::OK)
}
