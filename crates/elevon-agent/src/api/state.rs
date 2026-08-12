use std::path::PathBuf;

use anyhow::Result;
use elevon_fs::agent::get_socket_path;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::RwLock;

use crate::env::ElevonEnv;
use crate::proxy::types::AgentEvent;

pub struct AppState {
    pub db: toasty::Db,
    pub env: RwLock<ElevonEnv>,
    pub socket_client: SocketClient,
}

impl AppState {
    pub async fn new(env: &ElevonEnv) -> Result<Self> {
        let db = crate::db::init_db(env.turso_remote_url.as_deref()).await?;
        let env = RwLock::new(env.clone());
        Ok(Self {
            db,
            env,
            socket_client: SocketClient::new(),
        })
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

    pub async fn send(&self, mut stream: UnixStream, event: AgentEvent) -> Result<()> {
        let payload = serde_json::to_vec(&serde_json::json!(event))?;
        stream.write_all(&payload).await?;
        Ok(())
    }

    pub async fn shutdown(&self, mut stream: UnixStream) -> Result<()> {
        stream.shutdown().await?;
        Ok(())
    }
}
