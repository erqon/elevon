pub mod app;
mod env;
pub mod registry;

use std::collections::HashMap;

use elevon_config::ElevonConfig;
use serde::Deserialize;

use crate::config::app::AppConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,

    pub registry: registry::RegistryConfig,

    #[serde(default)]
    pub env: env::EnvConfig,

    #[serde(default, flatten)]
    pub app_config: Option<AppConfig>,

    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

impl Config {}

impl ElevonConfig for Config {}
