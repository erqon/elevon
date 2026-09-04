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
    if matches!(command, Commands::Init) {
        return cli::init::run();
    }

    let config = Config::from_file(&arg.config)?;
    let agent_credentials = config.elevon.agent.resolved_credentials()?;
    let agent_client = AgentClient::new(&agent_credentials.url, &agent_credentials.key)?;

    match command {
        Commands::Init => unreachable!(),
        Commands::Check => {
            tracing::info!("Successfully passed config file check {}", &arg.config);
        }
        Commands::Build(args) => {
            config.run_build(&arg.config, args.push).await?;
        }
        Commands::Push => {
            config.run_push().await?;
        }
        Commands::Deploy(args) => {
            let registry_credentials = config.registry.resolved_credentials()?;

            if args.build {
                config.run_build(&arg.config, true).await?;
            }

            config
                .run_deploy(&agent_client, &args.apps, registry_credentials)
                .await?;
        }
        Commands::Rollback(args) => {
            config.run_rollback(&agent_client, &args.apps).await?;
        }
    }

    Ok(())
}
