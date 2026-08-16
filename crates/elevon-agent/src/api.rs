pub mod db;
mod routes;
pub mod state;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::Router;

use crate::env::ElevonEnv;

pub async fn run_api_server(env: &ElevonEnv) -> Result<()> {
    let state = state::AppState::new(env).await?;
    let router = routes::create_router(state);
    let app = Router::new().nest("/api/v1", router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind API listener to 127.0.0.1:3000")?;

    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("API server failed")?;

    Ok(())
}
