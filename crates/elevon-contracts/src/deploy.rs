use serde::{Deserialize, Serialize};

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
    pub name: String,
    pub image: String,
    pub role: AppRole,
    pub web_app: Option<WebApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppReleasePayload {
    pub apps: Vec<AppPayload>,
}
