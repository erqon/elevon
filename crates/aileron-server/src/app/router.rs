use crate::{
    app::{
        error::{AppError, AppJson},
        state::AppState,
    },
    db::models::user::User,
    routes,
    services::auth::{AuthUser, require_auth},
};
use axum::{
    Router,
    extract::State,
    middleware::{from_fn, from_fn_with_state},
    routing::get,
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::routes::auth_router;

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/me", get(me))
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/health", get(routes::health::health))
        .nest("/auth", auth_router())
        .nest("/api", protected)
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
    State(mut state): State<AppState>,
    AuthUser { user_id }: AuthUser,
) -> anyhow::Result<AppJson<serde_json::Value>, AppError> {
    let user = match User::get_by_id(&mut state.db, user_id).await {
        Ok(user) => user,
        Err(err) if err.is_record_not_found() => return Err(AppError::Unauthorized),
        Err(err) => return Err(AppError::Db(err)),
    };

    Ok(AppJson(serde_json::json!({
        "id": user.id,
        "email": &user.email,
        "first_name": &user.first_name,
        "last_name": &user.last_name,
        "created_at": &&user.created_at.to_string()
    })))
}
