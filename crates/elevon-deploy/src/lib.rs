use anyhow::Context;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};

use crate::{
    agent::AgentClient,
    cli::{Commands, DeployArgs},
    config::Config,
};

pub mod agent;
pub mod cli;
pub mod config;
pub mod image;
pub mod util;

pub async fn run_deploy_cli(arg: DeployArgs, command: Commands) -> anyhow::Result<()> {
    let with_level = matches!(
        command,
        Commands::Build(_) | Commands::Deploy(_) | Commands::Rollback(_)
    );
    elevon_http::init_cli_logging(with_level);

    if matches!(command, Commands::Init) {
        return cli::init::run();
    }

    let config_path = match (&arg.config, &arg.project) {
        (Some(path), None) => path.clone(),
        (None, Some(project)) => format!(".elevon/deploy.{project}.yml"),
        (None, None) => ".elevon/deploy.yml".to_string(),
        (Some(_), Some(_)) => unreachable!("clap rejects conflicting arguments"),
    };

    let config = Config::from_file(&config_path)?;
    let agent_credentials = config
        .elevon
        .agent
        .resolved_credentials()
        .context("failed to resolve agent credentials")?;

    let agent_client = AgentClient::new(&agent_credentials.url, &agent_credentials.key)
        .context("failed to create agent client")?;

    // TODO: Do the resolve_credentials in here so the `check` command will check the environment variables too
    let registry_credentials = config
        .registry
        .resolved_credentials()
        .context("failed to resolve registry credentials")?;

    match command {
        Commands::Init => unreachable!(),
        Commands::Check => {
            // TODO: Check for environment variables too
            tracing::info!("Successfully passed config file check {}", &config_path);
        }
        Commands::Build(args) => {
            config
                .run_build(&registry_credentials, &config_path, args.push)
                .await?;
        }
        Commands::Push => {
            config.run_push().await?;
        }
        Commands::Deploy(args) => {
            config
                .run_deploy(
                    &agent_client,
                    &args.apps,
                    registry_credentials,
                    &config_path,
                )
                .await?;
        }
        Commands::Rollback(args) => {
            config.run_rollback(&agent_client, &args.apps).await?;
        }
    }

    Ok(())
}
