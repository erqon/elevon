pub mod db;
pub mod event;
mod routes;
pub mod state;
pub mod stream;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;
use tokio::task::JoinSet;

use crate::{env::ElevonEnv, socket::Socket};

pub async fn run_api_server(env: &ElevonEnv) -> Result<()> {
    let state = Arc::new(state::ApiState::new(env).await?);
    let cloned_state = state.clone();

    let router = routes::create_router(cloned_state);
    let app = Router::new().merge(router);

    let mut set = JoinSet::new();

    Socket::create_api_listener_handle(&mut set, state.api_socket.clone(), state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind API listener to 127.0.0.1:3000")?;

    tracing::info!("listening on {}", listener.local_addr()?);

    tokio::select! {
        result = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        ) => {
            result.context("API server failed")?;
        }

        result = set.join_next() => {
            if let Some(result) = result {
                result?;
            }
        }
    }

    Ok(())
}
