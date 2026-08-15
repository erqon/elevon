use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppEnvPayload {
    pub name: String,
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppEnvSetPayload {
    pub apps: Vec<AppEnvPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppRole {
    Web,
    Worker,
}

impl std::default::Default for AppRole {
    fn default() -> Self {
        AppRole::Web
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebApp {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPayload {
    pub project: String,
    pub image: String,
    pub name: String,
    pub role: AppRole,
    pub web_app: Option<WebApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppReleasePayload {
    pub apps: Vec<AppPayload>,
}

pub fn format_app_env_name(project_name: &str, app_name: &str) -> String {
    format!("{}.{}", app_name, project_name)
}
