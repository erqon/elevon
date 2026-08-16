use std::{sync::atomic::AtomicUsize, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum AgentEvent {
    UpsertRoute(RouteConfig),
    DeleteRoute(DeleteRoute),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteState {
    Active,
    Draining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub port: u16,
    pub state: RouteState,
    pub container_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoute {
    pub id: String,
    pub domain: String,
}

#[derive(Debug)]
pub struct BackendRuntime {
    pub state: RouteState,
    pub inflight: AtomicUsize,
    pub drain_started_at: Option<Instant>,
}

impl std::default::Default for BackendRuntime {
    fn default() -> Self {
        BackendRuntime {
            state: RouteState::Active,
            inflight: AtomicUsize::new(0),
            drain_started_at: None,
        }
    }
}
