use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Instant,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum AgentEvent {
    GetRunningContainers(Vec<RouteConfig>),
    UpsertRoute(RouteConfig),
    DeleteRoute(DeleteRoute),
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
    pub container_id: String,
    pub state: RouteState,
    pub port: u16,
    pub inflight: AtomicUsize,
    pub drain_started_at: Option<Instant>,
}

impl BackendRuntime {
    pub fn new(container_id: String, port: u16) -> Arc<Self> {
        Arc::new(BackendRuntime {
            container_id,
            state: RouteState::Active,
            port,
            inflight: AtomicUsize::new(0),
            drain_started_at: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteState {
    Active,
    Draining,
}
