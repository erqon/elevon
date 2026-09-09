use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        db::models::{AuthKey, AuthKeyTableRow},
        state::SharedApiState,
    },
    cli::key::KeyCommands,
    proxy::types::DeployAppData,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEvent {
    RunningContainers,
    KeyCommands(KeyCommands),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEventResponse {
    RunningContainers(Vec<DeployAppData>),
    KeyCreate(String),
    KeyList(Vec<AuthKeyTableRow>),
}

impl ApiSocketEvent {
    pub async fn handle(
        event: ApiSocketEvent,
        state: SharedApiState,
    ) -> Result<Option<ApiSocketEventResponse>> {
        let response: Option<ApiSocketEventResponse> = match event {
            ApiSocketEvent::RunningContainers => {
                let result = state.get_running_route_containers().await?;
                Some(ApiSocketEventResponse::RunningContainers(result))
            }
            ApiSocketEvent::KeyCommands(command) => match command {
                KeyCommands::Create(args) => {
                    let result = AuthKey::create_key(&mut state.db.get(), &args.name).await?;
                    Some(ApiSocketEventResponse::KeyCreate(result))
                }
                KeyCommands::List => {
                    let result = AuthKey::all().exec(&mut state.db.get()).await?;
                    let rows: Vec<AuthKeyTableRow> = result
                        .into_iter()
                        .map(|key| AuthKeyTableRow {
                            id: key.id.to_string(),
                            name: key.name,
                            enabled: key.enabled,
                            expires: key.expires_at.to_string(),
                            last_used: key
                                .last_used_at
                                .map(|timestamp| timestamp.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                            revoked: key
                                .revoked_at
                                .map(|timestamp| timestamp.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        })
                        .collect();

                    Some(ApiSocketEventResponse::KeyList(rows))
                }
                _ => None,
            },
        };

        Ok(response)
    }
}
