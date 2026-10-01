use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use tabled::{Table, settings::Style};

use crate::{
    api::{
        db::models::{AuthKey, AuthKeyTabled, TabledView},
        event::{ApiSocketEvent, ApiSocketEventResponse},
        state::SharedApiState,
    },
    cli::CliComponent,
    logger::ActionLogger,
    socket::{Emitter, SocketEventHandler},
};

#[derive(Args, Serialize, Deserialize)]
pub struct KeyCreateArgs {
    #[arg(
        value_name = "NAME",
        default_value = "Default",
        help = "Name of the key"
    )]
    pub name: String,
}

#[derive(Args, Clone, Serialize, Deserialize)]
pub struct KeyRevokeArgs {
    #[arg(
        value_name = "KEY",
        required = true,
        help = "A key ID/name or list of key IDs/names to revoke"
    )]
    pub keys: Vec<String>,
}

#[derive(Args, Clone, Serialize, Deserialize)]
pub struct KeyDeleteArgs {
    #[arg(
        value_name = "KEY",
        required = true,
        help = "A key ID/name or list of key IDs/names to delete"
    )]
    pub keys: Vec<String>,

    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum KeyCommands {
    #[command(about = "Create new auth keys")]
    Create(KeyCreateArgs),

    #[command(about = "List all keys")]
    List,

    #[command(about = "Revoke one or more keys")]
    Revoke(KeyRevokeArgs),

    #[command(about = "Delete one or more keys")]
    Delete(KeyDeleteArgs),
}

impl CliComponent for KeyCommands {
    async fn run(command: Self) -> Result<()> {
        let api_socket = Self::get_socket()?;

        // TODO: Switch the whole cli commands to axum unix socket just like deploy communcates with it.
        match command {
            KeyCommands::Create(args) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Create(args)))
                    .await
                    .context("failed to contact the API server through api.sock")?;

                if let Some(response) = response
                    && let ApiSocketEventResponse::KeyResponse(KeySocketResponse::KeyCreate(key)) =
                        response
                {
                    tracing::info!("Save your API Key: {}", key);
                }
            }
            KeyCommands::List => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::List))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::KeyResponse(KeySocketResponse::KeyList(rows))) =
                    response
                {
                    tracing::info!("{}", Table::new(rows).with(Style::modern()));
                }
            }
            KeyCommands::Revoke(args) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Revoke(
                        args.clone(),
                    )))
                    .await?;

                if matches!(
                    response,
                    Some(ApiSocketEventResponse::KeyResponse(
                        KeySocketResponse::KeyUpdated
                    ))
                ) {
                    tracing::info!("Successfully revoked all keys");
                }
            }
            KeyCommands::Delete(args) => {
                let formatted_keys: String = args
                    .keys
                    .iter()
                    .map(|id| format!("- {}", id))
                    .collect::<Vec<_>>()
                    .join("\n");

                let message = [
                    "This will delete auth key(s):",
                    "",
                    &formatted_keys,
                    "",
                    "Continue? [y/N]",
                ]
                .join("\n");

                elevon_contracts::handle_cli_yes(args.yes, message)?;

                let _: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Delete(
                        args.clone(),
                    )))
                    .await?;
            }
        }

        Ok(())
    }
}

impl SocketEventHandler for KeyCommands {
    type Command = KeyCommands;
    type Response = KeySocketResponse;
    type EventRespose = ApiSocketEventResponse;

    async fn handle_event(
        command: Self::Command,
        state: SharedApiState,
        emitter: Emitter<Self::EventRespose>,
    ) -> Result<Option<Self::Response>> {
        match command {
            KeyCommands::Create(args) => {
                emitter
                    .log(format!("Creating auth key: {}", args.name))
                    .await?;
                let result = AuthKey::create_key(&mut state.db.get(), &args.name).await?;
                Ok(Some(KeySocketResponse::KeyCreate(result)))
            }
            KeyCommands::List => {
                let data = handle_key_list(&mut state.db.get()).await?;
                Ok(Some(KeySocketResponse::KeyList(data)))
            }
            KeyCommands::Revoke(args) => {
                emitter.log("Revoking auth key(s)").await?;
                handle_key_revoke(args.keys, &mut state.db.get(), &emitter).await?;
                Ok(Some(KeySocketResponse::KeyUpdated))
            }
            KeyCommands::Delete(args) => {
                handle_key_delete(args.keys, &mut state.db.get(), &emitter)
                    .await
                    .context("auth key deletion failed")?;
                Ok(Some(KeySocketResponse::KeyUpdated))
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "key", content = "data")]
pub enum KeySocketResponse {
    KeyCreate(String),
    KeyList(Vec<AuthKeyTabled>),
    KeyUpdated,
}

pub async fn handle_key_list(db: &mut toasty::Db) -> Result<Vec<AuthKeyTabled>> {
    let result = AuthKey::all().exec(db).await?;
    let rows: Vec<AuthKeyTabled> = result.into_iter().map(|key| key.to_tabled()).collect();
    Ok(rows)
}

pub async fn handle_key_revoke(
    keys: Vec<String>,
    db: &mut toasty::Db,
    logger: &impl ActionLogger,
) -> Result<()> {
    for key in keys {
        let mut auth_key = AuthKey::get_by_name_or_id(db, &key)
            .await
            .context("failed to find key")?;

        if auth_key.revoked_at.is_some() {
            bail!("key is already revoked");
        }

        auth_key
            .update()
            .enabled(false)
            .revoked_at(jiff::Timestamp::now())
            .exec(db)
            .await?;

        logger.log(format!("Revoked auth key: '{key}'")).await?;
    }

    Ok(())
}

pub async fn handle_key_delete(
    keys: Vec<String>,
    db: &mut toasty::Db,
    logger: &impl ActionLogger,
) -> Result<()> {
    for key in keys {
        let auth_key = AuthKey::get_by_name_or_id(db, &key)
            .await
            .context("failed to find key")?;

        auth_key
            .delete()
            .exec(db)
            .await
            .context(format!("failed to delete key: {key}"))?;

        logger.log(format!("Deleted auth key: '{key}'")).await?;
    }

    Ok(())
}
