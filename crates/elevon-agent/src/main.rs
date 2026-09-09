use anyhow::{Context, Result};
use clap::Parser;
use elevon_agent::{
    cli::{Cli, Commands, key::KeyCommands},
    env::ElevonEnv,
};
use elevon_http::runtime::run_async;

fn main() {
    elevon_http::init_cli_logging();

    if let Err(err) = run() {
        tracing::error!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let env = ElevonEnv::load().context("failed to load agent environment")?;

    match cli.command {
        Commands::Init => {
            elevon_agent::cli::init::run().context("init command failed")?;
        }
        Commands::Install(args) => {
            run_async(elevon_agent::cli::install::run(cli.args, args))
                .context("install command failed")?;
        }
        Commands::Uninstall(args) => {
            elevon_agent::cli::install::uninstall(args).context("uninstall command failed")?;
        }
        Commands::Api => {
            run_async(elevon_agent::api::run_api_server(&env)).context("api command failed")?;
        }
        Commands::Proxy => {
            elevon_agent::proxy::run_proxy(&env).context("proxy command failed")?;
        }
        Commands::Key { subcommand } => {
            run_async(KeyCommands::run(subcommand)).context("key command failed")?;
        }
    }

    Ok(())
}
