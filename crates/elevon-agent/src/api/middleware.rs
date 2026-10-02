use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State, connect_info},
    middleware::Next,
    response::Response,
    serve::IncomingStream,
};
use elevon_http::{auth::get_auth_token, error::AppError};
use tokio::net::UnixListener;

use crate::api::{db::models::auth::check_auth_key, state::SharedApiState};

pub async fn auth_middleware(
    State(state): State<SharedApiState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = get_auth_token(req.headers())?;

    let db = state.db.get();
    let auth_key = check_auth_key(db, token).await?;

    req.extensions_mut().insert(auth_key);

    Ok(next.run(req).await)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct UdsConnectInfo {
    pub peer_addr: Option<Arc<tokio::net::unix::SocketAddr>>,
    pub peer_cred: Option<tokio::net::unix::UCred>,
}

impl connect_info::Connected<IncomingStream<'_, UnixListener>> for UdsConnectInfo {
    fn connect_info(stream: IncomingStream<'_, UnixListener>) -> Self {
        let io = stream.io();

        let peer_addr = io.peer_addr().ok().map(Arc::new);
        let peer_cred = io.peer_cred().ok();

        Self {
            peer_addr,
            peer_cred,
        }
    }
}

pub async fn require_unix_socket(
    ConnectInfo(info): ConnectInfo<UdsConnectInfo>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(_peer_cred) = info.peer_cred {
        // TODO: Further restrict by specific Linux process ID / User ID
        return Ok(next.run(req).await);
    }

    Err(AppError::NotFound)
}
