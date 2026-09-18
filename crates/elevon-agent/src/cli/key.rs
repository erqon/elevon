use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use tabled::{Table, settings::Style};

use crate::{
    api::event::{ApiSocketEvent, ApiSocketEventResponse},
    socket::{Socket, SocketType},
};

#[derive(Subcommand, Serialize, Deserialize)]
pub enum KeyCommands {
    #[command(about = "Command to create new auth keys")]
    Create(KeyCreateArgs),

    List,

    Revoke,
}

impl KeyCommands {
    pub async fn run(agent_name: &str, command: KeyCommands) -> Result<()> {
        let api_socket = Socket::new(agent_name, SocketType::Api)?;

        match command {
            KeyCommands::Create(args) => {
                let name = args.name.clone();

                tracing::info!("creating auth key: {}", name);

                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::Create(args)))
                    .await
                    .context("failed to contact the API server through api.sock")?;

                let Some(response) = response else {
                    bail!("failed to create auth key for {}", name);
                };

                if let ApiSocketEventResponse::KeyCreate(key) = response {
                    tracing::info!("save your API Key: {}", key);
                }
            }
            KeyCommands::List => {
                let response: Option<ApiSocketEventResponse> = api_socket
                    .send_and_receive(ApiSocketEvent::KeyCommands(KeyCommands::List))
                    .await
                    .context("failed to list auth keys")?;

                if let Some(ApiSocketEventResponse::KeyList(rows)) = response {
                    println!("{}", Table::new(rows).with(Style::modern()));
                }
            }
            KeyCommands::Revoke => {}
        }

        Ok(())
    }
}

#[derive(Args, Serialize, Deserialize)]
pub struct KeyCreateArgs {
    #[arg(long, short, help = "A name for the key", default_value = "Default")]
    pub name: String,
}
