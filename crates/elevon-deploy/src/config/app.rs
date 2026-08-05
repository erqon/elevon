use serde::Deserialize;

use crate::config::shared::RoutingConfig;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub image: String,
    #[serde(default)]
    pub routing: Option<RoutingConfig>,
}
