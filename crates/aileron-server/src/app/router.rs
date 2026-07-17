use crate::{
    app::{
        error::{AppError, AppJson},
        state::AppState,
    },
    routes,
    services::auth::AuthUser,
};
use axum::{Router, middleware::from_fn, routing::get};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::routes::auth_router;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health::health))
        .route("/me", get(me))
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

async fn me(
    AuthUser { user, session_id }: AuthUser,
) -> anyhow::Result<AppJson<serde_json::Value>, AppError> {
    Ok(AppJson(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "created_at": user.created_at.to_string(),
        "session_id": session_id,
    })))
}
