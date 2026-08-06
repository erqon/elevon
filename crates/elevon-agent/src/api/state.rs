use std::path::PathBuf;

use anyhow::Result;
use elevon_fs::agent::get_socket_path;
use tokio::net::UnixStream;

#[derive(Clone)]
pub struct AppState {
    pub socket_client: SocketClient,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            socket_client: SocketClient::new(),
        }
    }
}

#[derive(Clone)]
pub struct SocketClient {
    pub socket_path: PathBuf,
}

impl SocketClient {
    pub fn new() -> Self {
        Self {
            socket_path: get_socket_path(),
        }
    }

    pub async fn connect(&self) -> Result<UnixStream> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        Ok(stream)
    }

    // pub async fn send(&self, event_name: &str, data: ) {

    // }
}
