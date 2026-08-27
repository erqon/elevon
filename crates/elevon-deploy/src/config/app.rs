use bollard::plugin::RestartPolicyNameEnum;
use elevon_contracts::deploy::{AppRole, TlsConfig};
use serde::Deserialize;

use crate::config::env::EnvConfig;

#[derive(Debug, Default, Deserialize, Clone)]
pub struct AppConfig {
    pub role: AppRole,

    pub cmd: Option<String>,

    pub runtime: Option<AppRuntimeConfig>,

    pub env: Option<EnvConfig>,

    #[serde(skip)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppRuntimeConfig {
    pub restart: RestartPolicyNameEnum,
}
