use std::sync::Arc;

use axum::{Json, Router, http::StatusCode, routing::put};
use elevon_contracts::deploy::{AppEnvSetPayload, TlsType};
use elevon_fs::agent::{add_app_env, write_tls_file};
use elevon_http::error::AppError;

use crate::api::{db::models::AuthKey, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", put(set))
}

async fn set(_: AuthKey, Json(payload): Json<AppEnvSetPayload>) -> Result<StatusCode, AppError> {
    for app in payload.apps {
        for (key, value) in app.vars {
            match key.as_str() {
                "TLS_CERT" => write_tls_file(&app.project, &value, TlsType::Cert)?,
                "TLS_KEY" => write_tls_file(&app.project, &value, TlsType::Key)?,
                _ => add_app_env(&app.name, None, key.clone(), value.clone())?,
            }
        }
    }

    Ok(StatusCode::OK)
}
