mod env;
mod registry;

use elevon_config::{ConfigError, ElevonConfig, ResolveEnv};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub image: String,

    pub registry: registry::RegistryConfig,
    pub env: env::EnvConfig,
}

impl ResolveEnv for Config {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.registry.resolve_env()?;
        self.env.resolve_env()?;

        Ok(())
    }
}

impl ElevonConfig for Config {}
