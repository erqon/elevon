use std::sync::Arc;

use axum::{Json, Router, http::StatusCode, routing::put};
use elevon_contracts::deploy::{AppEnvSetPayload, TlsType, resolve_app_env_name};
use elevon_fs::agent::{add_app_env, write_tls_file};
use elevon_http::error::AppError;

use crate::api::{db::models::AuthKey, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", put(set))
}

async fn set(_: AuthKey, Json(payload): Json<AppEnvSetPayload>) -> Result<StatusCode, AppError> {
    for app in payload.apps {
        let resolved_path = resolve_app_env_name(&app.project, &app.name);

        if let Some(tls) = app.tls {
            write_tls_file(
                &resolved_path,
                tls.cert.as_bytes(),
                TlsType::Cert,
                Some(true),
            )?;
            write_tls_file(&resolved_path, tls.key.as_bytes(), TlsType::Key, Some(true))?;
        }

        for (key, value) in app.vars {
            add_app_env(&resolved_path, None, key.clone(), value.clone())?;
        }
    }

    Ok(StatusCode::OK)
}
