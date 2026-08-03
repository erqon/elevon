use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, net::UnixListener};

use crate::proxy::state::{ProxyState, RouteConfig};

pub struct SocketControl {
    pub state: Arc<ProxyState>,
}

#[async_trait]
impl BackgroundService for SocketControl {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        tokio::select! {
            _ = shutdown.changed() => {}
                _ = run_socket_listener(self.state.clone()) => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    event: String,
    #[serde(default)]
    data: serde_json::Value,
}

async fn run_socket_listener(state: Arc<ProxyState>) -> Result<(), Box<dyn Error>> {
    let socket = elevon_fs::agent::get_socket_path();

    let listener = UnixListener::bind(&socket)?;
    tracing::info!(path = %socket.display(), "socket listener ready");

    loop {
        let (mut stream, _) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            let mut buf = vec![];
            if let Err(err) = stream.read_to_end(&mut buf).await {
                tracing::error!(%err, "failed to read socket");
                return;
            }

            let message: Message = match serde_json::from_slice(&buf) {
                Ok(m) => m,
                Err(err) => {
                    tracing::error!(%err, "invalid json");
                    return;
                }
            };

            match message.event.as_str() {
                "upsert_route" => match serde_json::from_value::<RouteUpsert>(message.data) {
                    Ok(route) => {
                        state.upsert_route(
                            route.name,
                            RouteConfig {
                                id: route.id,
                                host: route.host,
                                port: route.port,
                            },
                        );
                        tracing::info!("route upserted");
                    }
                    Err(err) => tracing::error!(%err, "bad upsert_route data"),
                },
                other => tracing::warn!(event = other, "unknown event"),
            }
        });
    }
}

#[derive(Debug, Deserialize)]
struct RouteUpsert {
    name: String, // Host header key, e.g. "app.local"
    id: String,
    host: String,
    port: u16,
}
