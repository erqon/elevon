use elevon_config::{ResolveEnvCredentials, resolve_env_or_literal};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ElevonConfig {
    pub agent: AgentConfig,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub url: String,
    pub key: String,
}

impl ResolveEnvCredentials for AgentConfig {
    type Output = Self;

    fn resolved_credentials(&self) -> Result<Self::Output, elevon_config::ConfigError> {
        Ok(Self {
            url: resolve_env_or_literal(&self.url)?,
            key: resolve_env_or_literal(&self.key)?,
        })
    }
}
