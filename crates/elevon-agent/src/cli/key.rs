use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use tabled::{Table, settings::Style};

use crate::{
    api::event::{ApiSocketEvent, ApiSocketEventResponse},
    socket::{Socket, SocketType},
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
        value_name = "ID",
        required = true,
        help = "An ID or list of IDs of the keys to revoke"
    )]
    pub ids: Vec<String>,
}

#[derive(Args, Clone, Serialize, Deserialize)]
pub struct KeyDeleteArgs {
    #[arg(
        value_name = "ID",
        required = true,
        help = "An ID or list of IDs of the keys to delete"
    )]
    pub ids: Vec<String>,

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

impl KeyCommands {
    pub async fn run(command: KeyCommands) -> Result<()> {
        let api_socket = Socket::new(SocketType::Api)?;

        match command {
            KeyCommands::Create(args) => {
                let name = args.name.clone();

                tracing::info!("Creating auth key: {}", name);

                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Create(args)))
                    .await
                    .context("failed to contact the API server through api.sock")?;

                if let Some(response) = response
                    && let ApiSocketEventResponse::KeyCreate(key) = response
                {
                    tracing::info!("Save your API Key: {}", key);
                }
            }
            KeyCommands::List => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::List))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::KeyList(rows)) = response {
                    tracing::info!("{}", Table::new(rows).with(Style::modern()));
                }
            }
            KeyCommands::Revoke(args) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Revoke(
                        args.clone(),
                    )))
                    .await?;

                if matches!(response, Some(ApiSocketEventResponse::KeyUpdated)) {
                    tracing::info!("Successfully revoked all keys");
                }
            }
            KeyCommands::Delete(args) => {
                let formatted_keys: String = args
                    .ids
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

                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Delete(
                        args.clone(),
                    )))
                    .await?;

                if matches!(response, Some(ApiSocketEventResponse::KeyUpdated)) {
                    tracing::info!("Successfully deleted key(s): [{}]", args.ids.join(", "));
                }
            }
        }

        Ok(())
    }
}
