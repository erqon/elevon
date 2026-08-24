use std::collections::HashMap;

use anyhow::Result;
use bytes::Bytes;
use elevon_config::{ConfigError, ResolveEnvCredentials, resolve_env_or_literal};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppEnvPayload {
    pub project: String,
    pub name: String,
    pub vars: HashMap<String, String>,
    pub tls: Option<TlsConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}

impl ResolveEnvCredentials for TlsConfig {
    type Output = Self;

    fn resolved_credentials(&self) -> Result<Self::Output, ConfigError> {
        Ok(Self {
            cert: resolve_env_or_literal(&self.cert)?,
            key: resolve_env_or_literal(&self.key)?,
        })
    }
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

pub fn resolve_app_env_name(project_name: &str, app_name: &str) -> String {
    format!("{}/{}", project_name, app_name)
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
                    StreamEvent::Error { message } => {
                        tracing::error!(message);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
