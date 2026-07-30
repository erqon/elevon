mod registry;

use aileron_config::{AileronConfig, ConfigError, ResolveEnv, resolve_env_or_literal};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub image: String,

    pub registry: registry::RegistryConfig,
    pub env: std::collections::BTreeMap<String, String>,
}

impl ResolveEnv for Config {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.registry.resolve_env()?;

        for value in self.env.values_mut() {
            *value = resolve_env_or_literal(std::mem::take(value))?;
        }
        Ok(())
    }
}

impl AileronConfig for Config {}
