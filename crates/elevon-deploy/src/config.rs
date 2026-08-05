mod app;
mod env;
mod registry;
mod shared;

use std::collections::HashMap;

use elevon_config::{ConfigError, ElevonConfig, ResolveEnv};
use serde::Deserialize;

use crate::config::{app::AppConfig, shared::RoutingConfig};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,

    #[serde(default)]
    pub routing: Option<RoutingConfig>,

    pub registry: registry::RegistryConfig,
    #[serde(default)]
    pub env: env::EnvConfig,

    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

impl ResolveEnv for Config {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.registry.resolve_env()?;
        self.env.resolve_env()?;

        Ok(())
    }
}

impl ElevonConfig for Config {}
