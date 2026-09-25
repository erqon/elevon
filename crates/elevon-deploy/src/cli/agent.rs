use anyhow::Result;
use clap::{Args, Subcommand};
use elevon_contracts::deploy::log_stream_events;

use crate::agent::AgentClient;

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Auth key related commands")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },
}

impl Commands {
    pub async fn handle(&self, agent_client: AgentClient) -> Result<()> {
        match self {
            Commands::Key { command } => match command {
                KeyCommands::Create(args) => KeyCommands::create(&agent_client, args).await?,
                KeyCommands::List => KeyCommands::list(&agent_client).await?,
                KeyCommands::Revoke(args) => KeyCommands::revoke(&agent_client, args).await?,
                KeyCommands::Delete(args) => KeyCommands::delete(&agent_client, args).await?,
            },
        }

        Ok(())
    }
}

#[derive(Args)]
pub struct KeyCreateArgs {
    #[arg(
        value_name = "NAME",
        default_value = "Default",
        help = "Name of the key"
    )]
    pub name: String,
}

#[derive(Args)]
pub struct KeyRevokeArgs {
    #[arg(
        value_name = "ID",
        required = true,
        help = "An ID or list of IDs of the keys to revoke"
    )]
    pub ids: Vec<String>,
}

#[derive(Args)]
pub struct KeyDeleteArgs {
    #[arg(
        value_name = "ID",
        required = true,
        help = "An ID or list of IDs of the keys to delete"
    )]
    pub ids: Vec<String>,

    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum KeyCommands {
    #[command(about = "Create a new key")]
    Create(KeyCreateArgs),

    #[command(about = "List all keys")]
    List,

    #[command(about = "Revoke one or more keys")]
    Revoke(KeyRevokeArgs),

    #[command(about = "Delete one or more keys")]
    Delete(KeyDeleteArgs),
}

impl KeyCommands {
    async fn create(agent_client: &AgentClient, args: &KeyCreateArgs) -> Result<()> {
        let url = agent_client.absolute_url(&format!("/cli/key/{}", args.name));
        let headers = agent_client.headers();

        let event_stream = agent_client
            .client
            .post(url)
            .headers(headers)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        Ok(())
    }

    async fn list(agent_client: &AgentClient) -> Result<()> {
        let url = agent_client.absolute_url("/cli/key/list");
        let headers = agent_client.headers();

        let event_stream = agent_client
            .client
            .get(url)
            .headers(headers)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        Ok(())
    }

    async fn revoke(agent_client: &AgentClient, args: &KeyRevokeArgs) -> Result<()> {
        let url = agent_client.absolute_url("/cli/key/revoke");
        let headers = agent_client.headers();

        let payload = serde_json::json!({
            "data": args.ids,
        });

        let event_stream = agent_client
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        Ok(())
    }

    async fn delete(agent_client: &AgentClient, args: &KeyDeleteArgs) -> Result<()> {
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

        let url = agent_client.absolute_url("/cli/key/delete");
        let headers = agent_client.headers();

        let payload = serde_json::json!({
            "data": args.ids,
        });

        let event_stream = agent_client
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .bytes_stream();

        log_stream_events(event_stream).await?;

        Ok(())
    }
}
