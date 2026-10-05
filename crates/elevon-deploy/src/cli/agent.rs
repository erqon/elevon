use anyhow::Result;
use clap::{Args, Subcommand};
use elevon_contracts::{
    cli::{
        ListQueryParams,
        app::{AppCommands, AppStopCommandArgs, DeploymentCommands},
    },
    cli_verify_action_with_items,
    deploy::{AppStopPayload, log_stream_events},
};

use crate::{agent::AgentClient, config::Config};

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Auth key related commands")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    #[command(about = "App related commands")]
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
}

impl Commands {
    pub async fn handle(&self, agent_client: AgentClient, config: Config) -> Result<()> {
        match self {
            Commands::Key { command } => match command {
                KeyCommands::Create(args) => KeyCommands::create(&agent_client, args).await?,
                KeyCommands::List(params) => {
                    KeyCommands::list(&agent_client, "keys", Some(params)).await?
                }
                KeyCommands::Revoke(args) => KeyCommands::revoke(&agent_client, args).await?,
                KeyCommands::Delete(args) => KeyCommands::delete(&agent_client, args).await?,
            },

            Commands::App { command } => match command {
                AppCommands::List(params) => {
                    AppCommands::list(&agent_client, "apps", Some(params)).await?;
                }

                AppCommands::Deployment { subcommand } => match subcommand {
                    DeploymentCommands::List(params) => {
                        DeploymentCommands::list(&agent_client, "deployments", Some(params))
                            .await?;
                    }
                },

                AppCommands::Stop(args) => {
                    AppCommands::stop(&agent_client, &config, args).await?;
                }
            },
        }

        Ok(())
    }
}

#[derive(Subcommand)]
pub enum KeyCommands {
    #[command(about = "Create a new key")]
    Create(KeyCreateArgs),

    #[command(about = "List all keys")]
    List(ListQueryParams),

    #[command(about = "Revoke one or more keys")]
    Revoke(KeyRevokeArgs),

    #[command(about = "Delete one or more keys")]
    Delete(KeyDeleteArgs),
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

trait AgentCommandsTrait {
    fn list(
        agent_client: &AgentClient,
        base_endpoint: &str,
        query_params: Option<&ListQueryParams>,
    ) -> impl Future<Output = Result<()>> {
        async move {
            let url = agent_client.absolute_url(&format!("/cli/{base_endpoint}/list"));
            let headers = agent_client.headers();

            let mut req_builder = agent_client.client.get(url);

            if let Some(query_params) = query_params {
                req_builder = req_builder.query(query_params);
            }

            let event_stream = req_builder.headers(headers).send().await?.bytes_stream();

            log_stream_events(event_stream).await?;

            Ok(())
        }
    }
}

impl AgentCommandsTrait for KeyCommands {}

impl KeyCommands {
    async fn create(agent_client: &AgentClient, args: &KeyCreateArgs) -> Result<()> {
        let url = agent_client.absolute_url(&format!("/cli/keys/{}", args.name));
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

    async fn revoke(agent_client: &AgentClient, args: &KeyRevokeArgs) -> Result<()> {
        let url = agent_client.absolute_url("/cli/keys/revoke");
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
        cli_verify_action_with_items(args.yes, &args.ids, "This will delete auth key(s)")?;

        let url = agent_client.absolute_url("/cli/keys/delete");
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

trait AppCommandsTrait {
    async fn stop(
        agent_client: &AgentClient,
        config: &Config,
        args: &AppStopCommandArgs,
    ) -> Result<()> {
        cli_verify_action_with_items(
            args.force,
            &args.apps,
            "This will stop running appication(s)",
        )?;

        let url = agent_client.absolute_url("/cli/apps/stop");
        let headers = agent_client.headers();

        let payload = AppStopPayload {
            project: config.name.clone(),
            apps: args.apps.clone(),
            force: args.force,
        };

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

impl AgentCommandsTrait for AppCommands {}

impl AppCommandsTrait for AppCommands {}

impl AgentCommandsTrait for DeploymentCommands {}
