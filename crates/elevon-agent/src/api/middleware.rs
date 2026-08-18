use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use elevon_http::error::AppError;

pub async fn local_only(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !addr.ip().is_loopback() {
        return Err(AppError::client(
            StatusCode::FORBIDDEN,
            "forbidden",
            "local access only",
        ));
    }

    let response = next.run(request).await;

    Ok(response)
}
