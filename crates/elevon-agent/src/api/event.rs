use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{
        db::models::{AuthKey, AuthKeyTableRow},
        state::SharedApiState,
    },
    cli::key::{KeyCommands, KeyCreateArgs, KeyRevokeArgs},
    proxy::types::DeployAppData,
    socket::Emitter,
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
        emitter: Emitter<ApiSocketEventResponse>,
    ) -> Result<Option<ApiSocketEventResponse>> {
        let mut db = state.db.get();

        let response: Option<ApiSocketEventResponse> = match event {
            ApiSocketEvent::RunningContainers => {
                let result = state.get_running_route_containers().await?;
                Some(ApiSocketEventResponse::RunningContainers(result))
            }
            ApiSocketEvent::KeyCommands(command) => match command {
                KeyCommands::Create(args) => handle_key_create(args, &mut db, &emitter).await?,
                KeyCommands::List => handle_key_list(&mut db).await?,
                KeyCommands::Revoke(args) => {
                    handle_key_revoke(args, &mut db, &emitter).await?;
                    None
                }
                KeyCommands::Delete(_args) => None,
            },
        };

        Ok(response)
    }
}

async fn handle_key_create(
    args: KeyCreateArgs,
    db: &mut toasty::Db,
    emitter: &Emitter<ApiSocketEventResponse>,
) -> Result<Option<ApiSocketEventResponse>> {
    let result = AuthKey::create_key(db, &args.name).await;

    if let Err(_) = &result {
        emitter
            .log(&format!("Failed to create key, already exists with name: {}", args.name))
            .await?;
    }

    let result = result.context("failed to create auth key")?;

    Ok(Some(ApiSocketEventResponse::KeyCreate(result)))
}

async fn handle_key_list(db: &mut toasty::Db) -> Result<Option<ApiSocketEventResponse>> {
    let result = AuthKey::all().exec(db).await?;
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

    Ok(Some(ApiSocketEventResponse::KeyList(rows)))
}

async fn handle_key_revoke(
    args: KeyRevokeArgs,
    db: &mut toasty::Db,
    emitter: &Emitter<ApiSocketEventResponse>,
) -> Result<()> {
    for key in args.ids {
        let key_id = Uuid::parse_str(&key).context("failed to parse ID")?;
        let mut auth_key = AuthKey::get_by_id(db, key_id)
            .await
            .context("invalid ID key not found")?;

        if auth_key.revoked_at.is_some() {
            bail!("key is already revoked");
        }

        auth_key
            .update()
            .enabled(false)
            .revoked_at(jiff::Timestamp::now())
            .exec(db)
            .await?;

        emitter.log(&format!("Revoked auth key: {}", key)).await?;
    }

    Ok(())
}
