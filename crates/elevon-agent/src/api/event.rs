use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use uuid::Uuid;

use crate::{
    api::{db::models::AuthKey, state::SharedApiState},
    cli::key::KeyCommands,
    proxy::types::DeployAppData,
    socket::Emitter,
};

#[derive(Clone, Tabled, Serialize, Deserialize)]
pub struct AuthKeyTableRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub expires: String,
    pub last_used: String,
    pub revoked: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEvent {
    RunningContainers,
    KeyCommands(KeyCommands),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ApiSocketEventResponse {
    RunningContainers(Vec<DeployAppData>),
    KeyCreate(String),
    KeyList(Vec<AuthKeyTableRow>),
    KeyUpdated,
}

async fn emit_log(emitter: Emitter<ApiSocketEventResponse>, message: String) -> Result<()> {
    emitter.log(message).await?;
    Ok(())
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
            ApiSocketEvent::KeyCommands(command) => match command {
                KeyCommands::Create(args) => {
                    emitter
                        .log(format!("Creating auth key: {}", args.name))
                        .await?;
                    let result = AuthKey::create_key(&mut state.db.get(), &args.name).await?;
                    Some(ApiSocketEventResponse::KeyCreate(result))
                }
                KeyCommands::List => {
                    let data = handle_key_list(&mut state.db.get()).await?;
                    Some(ApiSocketEventResponse::KeyList(data))
                }
                KeyCommands::Revoke(args) => {
                    emitter.log("Revoking auth key(s)").await?;
                    handle_key_revoke(args.ids, &mut state.db.get(), move |message| {
                        emit_log(emitter.clone(), message)
                    })
                    .await?;
                    Some(ApiSocketEventResponse::KeyUpdated)
                }
                KeyCommands::Delete(args) => {
                    handle_key_delete(args.ids, &mut state.db.get(), move |message| {
                        emit_log(emitter.clone(), message)
                    })
                    .await
                    .context("auth key deletion failed")?;
                    Some(ApiSocketEventResponse::KeyUpdated)
                }
            },
        };

        Ok(response)
    }
}

pub async fn handle_key_list(db: &mut toasty::Db) -> Result<Vec<AuthKeyTableRow>> {
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

    Ok(rows)
}

pub async fn handle_key_revoke<F, Fut>(
    keys: Vec<String>,
    db: &mut toasty::Db,
    log_action: F,
) -> Result<()>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    for key in keys {
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

        log_action(format!("Revoked auth key: '{key}'")).await?;
    }

    Ok(())
}

pub async fn handle_key_delete<F, Fut>(
    keys: Vec<String>,
    db: &mut toasty::Db,
    log_action: F,
) -> Result<()>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    for key in keys {
        let key_id = Uuid::parse_str(&key).context("invalid UUID")?;
        let auth_key = AuthKey::get_by_id(db, key_id)
            .await
            .context("invalid ID, key not found")?;

        auth_key
            .delete()
            .exec(db)
            .await
            .with_context(|| format!("failed to delete key: {key}"))?;

        log_action(format!("Deleted auth key: '{key}'")).await?;
    }

    Ok(())
}
