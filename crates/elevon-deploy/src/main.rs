use std::collections::HashMap;

use anyhow::Result;
use clap::Parser;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};
use elevon_deploy::cli::{Cli, Commands};
use elevon_deploy::config::Config;
use elevon_deploy::config::app::AppConfig;

#[tokio::main]
async fn main() -> Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();
    let config = Config::from_file(&cli.config)?;

    match cli.command {
        Commands::Build(args) => {
            let apps = config.apps()?;
            let selected = select_apps(&apps, &args.apps)?;

            for (name, app) in selected {
                app.run_build(name, &config.registry.server, &cli.config)
                    .await?;
            }
        }
        Commands::Push(args) => {
            let apps = config.apps()?;
            let registry_credentials = config.registry.resolved_credentials()?;
            let selected = select_apps(&apps, &args.apps)?;

            for (name, app) in selected {
                app.run_push(
                    name,
                    registry_credentials.clone(),
                    args.build,
                    &config.registry.server,
                    &cli.config,
                )
                .await?;
            }
        }
        Commands::Check => {
            tracing::info!("Successfully passed config file check {}", &cli.config);
        }
    }

    Ok(())
}

fn select_apps<'a>(
    apps: &'a HashMap<String, &'a AppConfig>,
    names: &'a [String],
) -> Result<Vec<(&'a String, &'a AppConfig)>> {
    if names.is_empty() {
        Ok(apps.iter().map(|(n, a)| (n, *a)).collect())
    } else {
        names
            .iter()
            .map(|name| {
                apps.get(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown app `{name}`"))
                    .map(|app| (name, *app))
            })
            .collect()
    }
}
