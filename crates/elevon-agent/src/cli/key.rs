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

#[derive(Args, Clone, Serialize, Deserialize)]
pub struct KeyDeleteArgs {
    #[arg(name = "id", help = "An ID or list of IDs of the keys")]
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

    #[command(about = "Revoke keys")]
    Revoke(KeyRevokeArgs),

    #[command(about = "Delete keys")]
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
                    tracing::info!("\n{}", Table::new(rows).with(Style::modern()));
                }
            }
            KeyCommands::Revoke(args) => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Revoke(
                        args.clone(),
                    )))
                    .await?;

                if matches!(response, Some(ApiSocketEventResponse::KeyUpdated)) {
                    tracing::info!("Successfully revoked key(s): [{}]", args.ids.join(", "));
                }
            }
            KeyCommands::Delete(args) => {
                if !args.yes {
                    tracing::info!(
                        "This will delete auth key(s): [{}]. Continue? [y/N]",
                        args.ids.join(", ")
                    );

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                        tracing::info!("Cancelled");
                        return Ok(());
                    }
                }

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

#[derive(Args, Serialize, Deserialize)]
pub struct KeyCreateArgs {
    #[arg(long, short, help = "A name for the key", default_value = "Default")]
    pub name: String,
}
