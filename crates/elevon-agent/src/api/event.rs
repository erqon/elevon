use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    api::state::SharedApiState,
    cli::key::{KeyCommands, KeySocketResponse},
    proxy::types::DeployAppData,
    socket::Emitter,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEvent {
    RunningContainers,
    KeyCommands(KeyCommands),
    // AppCommands(AppCommands),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEventResponse {
    RunningContainers(Vec<DeployAppData>),
    KeysResponse(KeySocketResponse),
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
            ApiSocketEvent::KeyCommands(command) => Some(ApiSocketEventResponse::KeysResponse(
                KeyCommands::handle_event(command, state, emitter).await?,
            )),
        };

        Ok(response)
    }
}

pub trait ApiEvent {
    type Command;
    type Response;

    fn handle_event(
        command: Self::Command,
        state: SharedApiState,
        emitter: Emitter<ApiSocketEventResponse>,
    ) -> impl Future<Output = anyhow::Result<Self::Response>> + Send;

    fn emit_log(
        emitter: Emitter<ApiSocketEventResponse>,
        message: String,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        async move {
            emitter.log(message).await?;
            Ok(())
        }
    }
}
