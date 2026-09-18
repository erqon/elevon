use std::{sync::atomic::AtomicUsize, time::Instant};

use elevon_contracts::deploy::WebApp;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum AgentEvent {
    UpsertRoute(DeployAppData),
    DrainApp(DeployAppData),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeployAppState {
    #[default]
    Active,
    Draining,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeployAppData {
    pub id: String,
    pub agent: String,
    pub project: String,
    pub name: String,
    pub state: DeployAppState,
    pub container_id: String,
    pub web_app: Option<WebApp>,
}

#[derive(Debug, Default)]
pub struct RouteBackendRuntime {
    pub port: u16,
    pub inflight: AtomicUsize,
    pub drain_started_at: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct BackendRuntime {
    pub container_id: String,
    pub state: DeployAppState,
    pub route: Option<RouteBackendRuntime>,
}
