use elevon_config::{ResolveEnvCredentials, resolve_env_or_literal};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct ElevonConfig {
    pub agent: AgentConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "AgentConfig::default_name")]
    pub name: String,
    pub url: String,
    pub key: String,
}

impl ResolveEnvCredentials for AgentConfig {
    type Output = Self;

    fn resolved_credentials(&self) -> Result<Self::Output, elevon_config::ConfigError> {
        Ok(Self {
            name: self.name.to_string(),
            url: resolve_env_or_literal(&self.url)?,
            key: resolve_env_or_literal(&self.key)?,
        })
    }
}

impl AgentConfig {
    fn default_name() -> String {
        "default".to_string()
    }
}
