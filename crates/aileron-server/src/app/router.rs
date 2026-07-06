use crate::{app::state::AppState, routes};
use axum::{Router, middleware::from_fn, routing::get};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::routes::auth_router;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health::health))
        .nest("/auth", auth_router())
        .fallback(crate::app::error::not_found)
        .with_state(state)
        .layer(from_fn(crate::app::error::log_app_errors))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}
