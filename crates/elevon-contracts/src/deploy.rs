use core::fmt;
use std::collections::HashMap;

use anyhow::Result;
use bollard::plugin::RestartPolicyNameEnum;
use bytes::Bytes;
use elevon_config::{ConfigError, ResolveEnvCredentials, resolve_env_or_literal};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

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
    while let Some(chunk_result) = event_stream.next().await {
        let chunk = match chunk_result {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(error = %err, "Stream connection error");
                return Err(err.into());
            }
        };
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if line.starts_with(":") {
                continue;
            }

            if let Some(payload) = line.strip_prefix("data: ") {
                if payload == "[DONE]" {
                    tracing::debug!("Stream sent [DONE] payload");
                    continue;
                }

                match serde_json::from_str::<StreamEvent>(payload) {
                    Ok(StreamEvent::Log { message, .. }) => {
                        tracing::info!(message);
                    }
                    Ok(StreamEvent::Error { message }) => {
                        tracing::error!(message);
                    }
                    Ok(other_event) => {
                        tracing::debug!(?other_event, "Unhandled stream event variant");
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            raw_payload = payload,
                            "Failed to parse SSE payload as StreamEvent"
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppRole {
    #[default]
    Web,
    Worker,
}

impl fmt::Display for AppRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppRole::Web => write!(f, "web"),
            AppRole::Worker => write!(f, "worker"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseAppRoleError;

impl std::str::FromStr for AppRole {
    type Err = ParseAppRoleError;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        match s {
            "web" => Ok(AppRole::Web),
            "worker" => Ok(AppRole::Worker),
            _ => Err(ParseAppRoleError),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebApp {
    pub port: u16,
    pub domain: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppRuntimeOptions {
    pub role: AppRole,
    pub cmd: Option<Vec<String>>,
    pub restart: Option<RestartPolicyNameEnum>,
    pub memory_limit: Option<i64>,
    pub cpu_limit: Option<i64>,
    pub network: Option<String>,
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

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPayload {
    pub project: String,
    pub image: String,
    pub name: String,
    pub keep_releases: u8,
    pub runtime_options: AppRuntimeOptions,
    pub vars: Option<HashMap<String, String>>,
    pub tls: Option<TlsConfig>,
    pub web_app: Option<WebApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppDeployPayload {
    pub apps: Vec<AppPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppRollback {
    pub project: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppRollbackPayload {
    pub apps: Vec<AppRollback>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TlsType {
    Cert,
    Key,
}

impl TlsType {
    pub fn get_file(&self) -> &str {
        match self {
            TlsType::Cert => "cert.pem",
            TlsType::Key => "key.pem",
        }
    }
}
