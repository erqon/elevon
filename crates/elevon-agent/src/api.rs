pub mod db;
pub mod event;
mod routes;
pub mod state;
pub mod stream;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::Router;
use tokio::task::JoinSet;

use crate::{
    cli::ApiArgs,
    env::ElevonEnv,
    socket::{Socket, SocketType},
};

pub async fn run_api_server(args: ApiArgs, env: &ElevonEnv) -> Result<()> {
    let state = Arc::new(state::ApiState::new(env).await?);
    let cloned_state = state.clone();

    let router = routes::create_router(cloned_state);
    let app = Router::new().merge(router);

    let mut set = JoinSet::new();

    Socket::create_api_listener_handle(&mut set, state.api_socket.clone(), state.clone());

    let addr = format!("127.0.0.1:{}", args.port);
    let socket_path = Socket::new(SocketType::Api)?;

    let tcp = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("failed to bind API listener to {}", addr))?;

    let unix = tokio::net::UnixListener::bind(&socket_path.path).context(format!(
        "failed to bind API listener to UNIX Socket {}",
        socket_path.path.display(),
    ))?;

    tracing::info!("listening on {}", tcp.local_addr()?);

    let tcp_app = app.clone();
    set.spawn(async move {
        if let Err(err) = axum::serve(
            tcp,
            tcp_app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            tracing::error!(error = %err, "API server failed");
        }
    });

    set.spawn(async move {
        if let Err(err) = axum::serve(unix, app).await {
            tracing::error!(error = %err, "API UNIX server failed");
        }
    });

    if let Some(result) = set.join_next().await {
        result?;
    }

    Ok(())
}
