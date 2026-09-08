use serde::{Deserialize, Serialize};

use crate::proxy::types::DeployAppData;

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEvent {
    RunningContainers,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEventResponse {
    RunningContainers(Vec<DeployAppData>),
}
