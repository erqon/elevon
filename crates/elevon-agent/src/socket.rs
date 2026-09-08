use std::os::unix::fs::PermissionsExt;
use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use elevon_fs::agent::AgentPath;
use futures_util::{Future, SinkExt, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub enum SocketType {
    Proxy,
    Api,
}

pub struct Socket {
    pub path: PathBuf,
}

impl Socket {
    pub fn new(ty: SocketType) -> Result<Self> {
        let path = match ty {
            SocketType::Proxy => AgentPath::ProxySocket.ensure_parent_dir()?,
            SocketType::Api => AgentPath::ApiSocket.ensure_parent_dir()?,
        };

        Ok(Self { path })
    }

    pub async fn send<M>(&self, msg: M) -> Result<()>
    where
        M: Serialize + Send + 'static,
    {
        let stream = UnixStream::connect(&self.path).await?;
        let mut writer = FramedWrite::new(stream, LengthDelimitedCodec::new());

        let payload_bytes = serde_json::to_vec(&msg)?;
        writer.send(payload_bytes.into()).await?;

        Ok(())
    }

    pub async fn listener<M, S, F, Fut>(&self, state: S, action: F) -> Result<()>
    where
        M: DeserializeOwned + Send + 'static,
        S: Clone + Send + Sync + 'static,
        F: Fn(S, M) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        tracing::info!(path = %self.path.display(), "binding socket");
        let listener = UnixListener::bind(&self.path)
            .with_context(|| format!("failed to bind {}", self.path.display()))?;

        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o660))?;

        let action = Arc::new(action);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let action = action.clone();
                    let state = state.clone();

                    tokio::spawn(async move {
                        let mut reader = FramedRead::new(stream, LengthDelimitedCodec::new());

                        while let Some(result) = reader.next().await {
                            match result {
                                Ok(frame) => {
                                    let message: M = match serde_json::from_slice(&frame) {
                                        Ok(m) => m,
                                        Err(err) => {
                                            tracing::error!(%err, "invalid json");
                                            continue; // Keep listening for next frames instead of breaking the loop
                                        }
                                    };

                                    if let Err(err) = action(state.clone(), message).await {
                                        tracing::error!(%err, "Action execution failed");
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Failed to read framed data from client");
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Fatal error accepting socket connection");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}
