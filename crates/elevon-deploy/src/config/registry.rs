use elevon_config::{ConfigError, ResolveEnvCredentials, resolve_env_or_literal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub server: String,
    pub username: String,
    pub password: String,
}

impl ResolveEnvCredentials for RegistryConfig {
    type Output = Self;

    fn resolved_credentials(&self) -> Result<Self::Output, ConfigError> {
        Ok(Self {
            server: self.server.clone(),
            username: resolve_env_or_literal(&self.username)?,
            password: resolve_env_or_literal(&self.password)?,
        })
    }
}
