use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub image: Option<String>,

    #[serde(default)]
    pub build: Option<BuildConfig>,

    #[serde(default)]
    pub routing: Option<RoutingConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct BuildOptions {
    pub path: String,
    #[serde(default)]
    pub dockerfile: Option<String>,
}
