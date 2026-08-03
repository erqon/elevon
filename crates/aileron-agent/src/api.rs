use std::{net::SocketAddr, path::PathBuf};

use axum::{Router, http::StatusCode, routing::post};
use serde_json::json;
use tokio::{io::AsyncWriteExt, net::UnixStream};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub async fn run_api_server() {
    let app = Router::new()
        .route("/test-socket", post(test_socket))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn test_socket() -> Result<StatusCode, (StatusCode, String)> {
    let socket = PathBuf::from("/tmp/aileron-agent.sock");
    let mut stream = UnixStream::connect(&socket)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let payload = serde_json::to_vec(&json!({
        "event": "upsert_route",
        "data": {
            "name": "app.local",
            "id": "2",
            "host": "127.0.0.1",
            "port": 3003
        }
    }))
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    stream
        .write_all(&payload)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    stream
        .shutdown()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
