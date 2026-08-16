pub mod elevon;
pub mod env;
pub mod registry;

use std::collections::HashMap;

use anyhow::Result;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};
use elevon_contracts::deploy::AppRole;
use serde::Deserialize;

use crate::{agent::AgentClient, config::env::EnvConfig};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub image: String,

    pub elevon: elevon::ElevonConfig,

    pub registry: registry::RegistryConfig,

    pub build: BuildConfig,

    pub routing: RoutingConfig,

    #[serde(default)]
    pub env: Option<EnvConfig>,

    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

impl ElevonConfig for Config {}

impl Config {
    pub async fn run_build(&self, config_path: &str, push: bool) -> Result<()> {
        crate::image::build_image(&self.image, &self.registry.server, &self.build, config_path)
            .await?;

        if push {
            self.run_push().await?;
        }

        Ok(())
    }

    pub async fn run_push(&self) -> Result<()> {
        let registry_credentials = self.registry.resolved_credentials()?;
        crate::image::push_image(&self.image, registry_credentials).await?;
        Ok(())
    }

    pub fn get_selected_apps(&self, arg_apps: &[String]) -> Result<Vec<(String, AppConfig)>> {
        if arg_apps.is_empty() {
            if self.apps.len() > 0 {
                return Ok(self
                    .apps
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect());
            } else {
                return Ok(vec![(
                    "web".to_string(),
                    AppConfig {
                        role: AppRole::Web,
                        env: None,
                    },
                )]);
            }
        }

        arg_apps
            .iter()
            .map(|name| {
                self.apps
                    .get(name)
                    .cloned()
                    .map(|cfg| (name.clone(), cfg))
                    .ok_or_else(|| anyhow::anyhow!("unknown app `{name}`"))
            })
            .collect()
    }

    pub async fn run_release(
        &self,
        agent_client: &AgentClient,
        app_names: &Vec<String>,
    ) -> Result<()> {
        let selected = self.get_selected_apps(app_names)?;
        agent_client.push_release(self, selected).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoutingConfig {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum BuildConfig {
    Path(String),
    Options(BuildOptions),
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfig::Path(".".to_string())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BuildOptions {
    pub path: String,
    #[serde(default)]
    pub dockerfile: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub role: AppRole,

    #[serde(default)]
    pub env: Option<EnvConfig>,
}
