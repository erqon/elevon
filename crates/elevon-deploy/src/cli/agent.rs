use anyhow::Result;
use clap::Subcommand;
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
                KeyCommands::List => KeyCommands::list(&agent_client).await?,
            },
        }

        Ok(())
    }
}

#[derive(Subcommand)]
pub enum KeyCommands {
    List,
}

impl KeyCommands {
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
}
