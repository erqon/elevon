use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    api::state::SharedApiState,
    cli::{
        app::{AppCommands, AppSocketResponse, DeploymentCommands, DeploymentSocketResponse},
        key::{KeyCommands, KeySocketResponse},
    },
    proxy::types::DeployAppData,
    socket::{Emitter, SocketEventHandler},
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEvent {
    RunningContainers,
    KeyCommands(KeyCommands),
    AppCommands(AppCommands),
    DeploymentCommands(DeploymentCommands),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEventResponse {
    RunningContainers(Vec<DeployAppData>),
    KeyResponse(KeySocketResponse),
    AppResponse(AppSocketResponse),
    DeploymentResponse(DeploymentSocketResponse),
}

impl ApiSocketEvent {
    pub async fn handle(
        event: ApiSocketEvent,
        state: SharedApiState,
        emitter: Emitter<ApiSocketEventResponse>,
    ) -> Result<Option<ApiSocketEventResponse>> {
        let response: Option<ApiSocketEventResponse> = match event {
            ApiSocketEvent::RunningContainers => {
                let result = state.get_running_route_containers().await?;
                Some(ApiSocketEventResponse::RunningContainers(result))
            }
            ApiSocketEvent::KeyCommands(command) => Some(ApiSocketEventResponse::KeyResponse(
                KeyCommands::handle_event(command, state, emitter).await?,
            )),
            ApiSocketEvent::AppCommands(command) => Some(ApiSocketEventResponse::AppResponse(
                AppCommands::handle_event(command, state, emitter).await?,
            )),
            ApiSocketEvent::DeploymentCommands(command) => {
                Some(ApiSocketEventResponse::DeploymentResponse(
                    DeploymentCommands::handle_event(command, state, emitter).await?,
                ))
            }
        };

        Ok(response)
    }
}
