pub mod db;
mod routes;
pub mod state;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;

use crate::env::ElevonEnv;

pub async fn run_api_server(env: &ElevonEnv) -> Result<()> {
    let state = Arc::new(state::AppState::new(env).await?);
    let router = routes::create_router(state.clone());
    let app = Router::new().nest("/api/v1", router);

    let startup_state = state.clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        if let Err(err) = startup_state.send_startup_data(shutdown_rx).await {
            tracing::error!(%err, "startup data task failed");
        }
    });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind API listener to 127.0.0.1:3000")?;

    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await
    .context("API server failed")?;

    let _ = shutdown_tx.send(true);

    Ok(())
}
