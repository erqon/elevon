mod dto;
mod router;
mod routes;
mod state;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::Router;

use crate::env::ElevonEnv;

pub async fn run_api_server(env: &ElevonEnv) -> Result<()> {
    let state = state::AppState::new(env).await?;
    let router = router::create_router(state);
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

// async fn test_socket() -> Result<StatusCode, (StatusCode, String)> {
//     let socket = PathBuf::from("/tmp/elevon-agent.sock");
//     let mut stream = UnixStream::connect(&socket)
//         .await
//         .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

//     let payload = serde_json::to_vec(&json!({
//         "event": "upsert_route",
//         "data": {
//             "name": "app.local",
//             "id": "2",
//             "host": "127.0.0.1",
//             "port": 3003
//         }
//     }))
//     .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

//     stream
//         .write_all(&payload)
//         .await
//         .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
//     stream
//         .shutdown()
//         .await
//         .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

//     Ok(StatusCode::NO_CONTENT)
// }
