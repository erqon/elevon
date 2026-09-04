pub mod app;
pub mod elevon;
pub mod env;
pub mod registry;

use std::collections::HashMap;

use anyhow::Result;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};
use elevon_contracts::deploy::{AppRole, TlsConfig};
use serde::Deserialize;

use crate::{
    agent::AgentClient,
    config::{app::AppConfig, env::EnvConfig, registry::RegistryConfig},
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RoutingConfig {
    pub domain: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
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
#[serde(default)]
pub struct BuildOptions {
    pub path: String,
    pub dockerfile: Option<String>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            path: ".".to_string(),
            dockerfile: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub image: String,

    #[serde(default = "Config::default_keep_releases")]
    pub keep_releases: Option<u8>,

    pub elevon: elevon::ElevonConfig,

    pub registry: registry::RegistryConfig,

    pub build: BuildConfig,

    pub routing: RoutingConfig,

    pub env: Option<EnvConfig>,

    pub apps: HashMap<String, AppConfig>,
}

impl ElevonConfig for Config {}

impl Config {
    pub fn default_keep_releases() -> Option<u8> {
        Some(5)
    }

    pub fn prepare_project_env_vars(
        &self,
        registry_config: RegistryConfig,
    ) -> Result<HashMap<String, String>> {
        let mut vars = self
            .env
            .as_ref()
            .map_or_else(|| Ok(HashMap::default()), |env| env.resolved_credentials())?;

        vars.extend(registry_config.vars());

        Ok(vars)
    }

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
            if !self.apps.is_empty() {
                return Ok(self
                    .apps
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            AppConfig {
                                tls: self.routing.tls.clone(),
                                ..v.clone()
                            },
                        )
                    })
                    .collect());
            } else {
                return Ok(vec![(
                    "web".to_string(),
                    AppConfig {
                        role: AppRole::Web,
                        tls: self.routing.tls.clone(),
                        ..Default::default()
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
                    .map(|cfg| {
                        (
                            name.clone(),
                            AppConfig {
                                tls: self.routing.tls.clone(),
                                ..cfg
                            },
                        )
                    })
                    .ok_or_else(|| anyhow::anyhow!("unknown app `{name}`"))
            })
            .collect()
    }

    pub async fn run_deploy(
        &self,
        agent_client: &AgentClient,
        app_names: &[String],
        registry_config: RegistryConfig,
    ) -> Result<()> {
        let selected = self.get_selected_apps(app_names)?;
        agent_client
            .push_deploy(self, selected, registry_config)
            .await?;
        Ok(())
    }

    pub async fn run_rollback(
        &self,
        agent_client: &AgentClient,
        app_names: &[String],
    ) -> Result<()> {
        let selected = self.get_selected_apps(app_names)?;
        agent_client.push_rollback(self, selected).await?;
        Ok(())
    }
}
