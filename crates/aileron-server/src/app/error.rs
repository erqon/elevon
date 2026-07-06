use std::sync::Arc;

use axum::{
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(AppError::from)?;
        Ok(Self(value))
    }
}

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(Debug)]
pub enum AppError {
    NotFound,
    Unauthorized,
    Db(toasty::Error),
    Internal(anyhow::Error),
    JsonRejection(JsonRejection),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            code: &'static str,
            message: String,
        }

        let err = Arc::new(self);
        let (status, code, message) = match &*err {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found".to_string()),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized".to_string(),
            ),
            AppError::Db(db_err) if db_err.is_record_not_found() => {
                (StatusCode::NOT_FOUND, "not_found", "Not found".to_string())
            }
            AppError::Db(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                "Database error".to_string(),
            ),
            AppError::Internal(internal_err) => {
                tracing::error!(error = %internal_err, "unhandled application error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal server error".to_string(),
                )
            }
            AppError::JsonRejection(rejection) => {
                (rejection.status(), "invalid_json", rejection.body_text())
            }
        };

        let mut response = (status, AppJson(ErrorResponse { code, message })).into_response();
        response.extensions_mut().insert(err);
        response
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<toasty::Error> for AppError {
    fn from(err: toasty::Error) -> Self {
        Self::Db(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

pub async fn not_found() -> AppError {
    AppError::NotFound
}

pub(crate) async fn log_app_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if let Some(err) = response.extensions().get::<Arc<AppError>>() {
        tracing::warn!(?err, "request failed");
    }
    response
}
