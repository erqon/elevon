use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use tabled::{Table, settings::Style};

use crate::{
    api::event::{ApiSocketEvent, ApiSocketEventResponse},
    socket::{Socket, SocketType},
};

#[derive(Args, Clone, Serialize, Deserialize)]
pub struct KeyRevokeArgs {
    #[arg(name = "id", help = "An ID or list of IDs of the keys")]
    pub ids: Vec<String>,
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum KeyCommands {
    #[command(about = "Create new auth keys")]
    Create(KeyCreateArgs),

    #[command(about = "List all keys")]
    List,

    #[command(about = "Revoke keys")]
    Revoke(KeyRevokeArgs),

    #[command(about = "Delete keys")]
    Delete(KeyRevokeArgs),
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
                api_socket
                    .send(ApiSocketEvent::KeyCommands(KeyCommands::Revoke(
                        args.clone(),
                    )))
                    .await?;

                tracing::info!("Successfully revoked key(s): [{}]", args.ids.join(", "));
            }
            KeyCommands::Delete(_args) => {}
        }

        Ok(())
    }
}

#[derive(Args, Serialize, Deserialize)]
pub struct KeyCreateArgs {
    #[arg(long, short, help = "A name for the key", default_value = "Default")]
    pub name: String,
}
