use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum AgentEvent {
    UpsertRoute(RouteConfig),
    DeleteRoute(DeleteRoute),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub id: String,
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoute {
    pub domain: String,
    pub id: String,
}
