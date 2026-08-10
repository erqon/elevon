use anyhow::Result;
use serde::Deserialize;

use crate::config::registry::RegistryConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub image: Option<String>,

    #[serde(default)]
    pub build: Option<BuildConfig>,

    #[serde(default)]
    pub routing: Option<RoutingConfig>,
}

impl AppConfig {
    pub async fn run_build(&self, name: &str, registry: &str, config_path: &str) -> Result<()> {
        let image = self.image(name)?.to_owned();
        let build = self.build(name)?.clone();

        crate::image::build_image(&image, registry, &build, config_path).await?;
        Ok(())
    }

    pub async fn run_push(
        &self,
        name: &str,
        creds: RegistryConfig,
        build_before: bool,
        registry: &str,
        config_path: &str,
    ) -> Result<()> {
        if build_before {
            self.run_build(name, registry, config_path).await?;
        }

        let image = self.image(name)?.to_owned();
        crate::image::push_image(&image, creds).await?;
        Ok(())
    }

    fn image(&self, name: &str) -> Result<&str> {
        self.image
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("app `{name}` is missing `image`"))
    }

    fn build(&self, name: &str) -> Result<&BuildConfig> {
        self.build
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("app `{name}` is missing `build`"))
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
