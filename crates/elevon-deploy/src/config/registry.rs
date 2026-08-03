use elevon_config::{resolve_env_or_literal, ConfigError, ResolveEnv};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegistryConfig {
    pub server: String,
    pub username: String,
    pub password: String,
}

impl ResolveEnv for RegistryConfig {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.username = resolve_env_or_literal(&self.username)?;
        self.password = resolve_env_or_literal(&self.password)?;
        Ok(())
    }
}
