use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub id: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRoute {
    pub domain: String,
    pub config: RouteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoute {
    pub domain: String,
    pub id: String,
}
