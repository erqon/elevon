use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use tokio::{io::AsyncReadExt, net::UnixListener};

use crate::proxy::state::ProxyState;
use crate::proxy::types::AgentEvent;

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

async fn run_socket_listener(state: Arc<ProxyState>) -> Result<(), Box<dyn Error>> {
    let socket = elevon_fs::agent::get_socket_path(true);

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

            let message: AgentEvent = match serde_json::from_slice(&buf) {
                Ok(m) => m,
                Err(err) => {
                    tracing::error!(%err, "invalid json");
                    return;
                }
            };

            match message {
                AgentEvent::GetRunningContainers(running_containers) => {
                    if let Err(err) = state.load_conainters(running_containers).await {
                        tracing::error!(%err, "failed to load containers");
                    }
                }
                AgentEvent::UpsertRoute(route) => {
                    let id = route.id.clone();
                    state.upsert_route(route);
                    tracing::info!("route {} upserted", &id);
                }
                AgentEvent::DeleteRoute(_route) => {}
            }
        });
    }
}
