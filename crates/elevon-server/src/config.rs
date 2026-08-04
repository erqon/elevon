use elevon_config::{ElevonConfig, ResolveEnv, resolve_env_or_literal};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

impl ResolveEnv for Config {
    fn resolve_env(&mut self) -> Result<(), elevon_config::ConfigError> {
        self.database.resolve_env()?;
        Ok(())
    }
}

impl ElevonConfig for Config {}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub remote_url: Option<String>,
}

impl ResolveEnv for DatabaseConfig {
    fn resolve_env(&mut self) -> Result<(), elevon_config::ConfigError> {
        self.path = resolve_env_or_literal(&self.path)?;
        if let Some(ref mut url) = self.remote_url {
            resolve_env_or_literal(url)?;
        }
        Ok(())
    }
}
