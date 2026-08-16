use anyhow::Result;
use clap::Parser;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};
use elevon_deploy::agent::AgentClient;
use elevon_deploy::cli::{Cli, Commands};
use elevon_deploy::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();
    let config = Config::from_file(&cli.config)?;

    let agent_credentials = config.elevon.agent.resolved_credentials()?;
    let agent_client = AgentClient::new(&agent_credentials.url, &agent_credentials.key)?;

    match cli.command {
        Commands::Build(args) => {
            config.run_build(&cli.config, args.push).await?;
        }
        Commands::Push => {
            config.run_push().await?;
        }
        Commands::Release(args) => {
            config.run_release(&agent_client, &args.apps).await?;
        }
        Commands::Env { args, subcommand } => {
            let selected = config.get_selected_apps(&args.apps)?;

            let registry_credentials = config.registry.resolved_credentials()?;
            let root_vars = match &config.env {
                Some(env_vars) => Some(env_vars.resolved_credentials()?),
                None => None,
            };
            let tls_vars = match config.routing.tls {
                Some(tls_vars) => Some(tls_vars.resolved_credentials()?),
                None => None,
            };

            subcommand
                .run(
                    &config.name,
                    &agent_client,
                    selected,
                    &registry_credentials,
                    root_vars,
                    tls_vars,
                )
                .await?;
        }
        Commands::Check => {
            tracing::info!("Successfully passed config file check {}", &cli.config);
        }
    }

    Ok(())
}
