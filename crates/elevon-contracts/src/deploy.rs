use std::collections::HashMap;

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppEnvPayload {
    pub project: String,
    pub name: String,
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppEnvSetPayload {
    pub apps: Vec<AppEnvPayload>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppRole {
    #[default]
    Web,
    Worker,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebApp {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPayload {
    pub project: String,
    pub image: String,
    pub name: String,
    pub role: AppRole,
    pub web_app: Option<WebApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppDeployPayload {
    pub apps: Vec<AppPayload>,
}

pub fn format_app_env_name(project_name: &str, app_name: &str) -> String {
    format!("{}.{}", app_name, project_name)
}

#[derive(PartialEq, Eq)]
pub enum TlsType {
    Cert,
    Key,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamLogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    Log {
        level: StreamLogLevel,
        message: String,
    },
    Done,
    Error {
        message: String,
    },
}

pub async fn log_stream_events<S>(mut event_stream: S) -> anyhow::Result<()>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    while let Some(chunk) = event_stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if line.starts_with(":") {
                continue;
            }

            if let Some(payload) = line.strip_prefix("data: ") {
                let event: StreamEvent = serde_json::from_str(payload)?;
                match event {
                    StreamEvent::Log { message, .. } => {
                        tracing::info!(message);
                    }
                    StreamEvent::Done => {
                        tracing::info!("Done");
                    }
                    StreamEvent::Error { message } => {
                        tracing::error!(message);
                    }
                }
            }
        }
    }

    Ok(())
}
