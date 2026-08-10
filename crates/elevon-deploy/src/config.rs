pub mod app;
pub mod elevon;
pub mod env;
pub mod registry;

use std::collections::HashMap;

use elevon_config::ElevonConfig;
use serde::Deserialize;

use crate::config::app::AppConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,

    pub elevon: elevon::ElevonConfig,

    pub registry: registry::RegistryConfig,

    #[serde(default, flatten)]
    pub app_config: Option<AppConfig>,

    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

impl Config {
    pub fn apps(&self) -> anyhow::Result<HashMap<String, &AppConfig>> {
        let has_root = self
            .app_config
            .as_ref()
            .is_some_and(|a| a.image.is_some() || a.build.is_some() || a.routing.is_some());
        let has_apps = !self.apps.is_empty();

        match (has_root, has_apps) {
            (true, true) => {
                anyhow::bail!("use either a root app (image/build/...) or `apps:`, not both")
            }
            (false, false) => anyhow::bail!("no app config found"),
            (true, false) => {
                let app = self.app_config.as_ref().unwrap();
                Ok(HashMap::from([(self.name.clone(), app)]))
            }
            (false, true) => Ok(self.apps.iter().map(|(k, v)| (k.clone(), v)).collect()),
        }
    }
}

impl ElevonConfig for Config {}
