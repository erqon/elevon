use aileron_config::{ConfigError, ResolveEnv, resolve_env_or_literal};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegistryConfig {
    pub server: String,
    pub username: String,
    pub password: String,
}

impl ResolveEnv for RegistryConfig {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.username = resolve_env_or_literal(std::mem::take(&mut self.username))?;
        self.password = resolve_env_or_literal(std::mem::take(&mut self.password))?;
        Ok(())
    }
}
