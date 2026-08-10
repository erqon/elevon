use anyhow::{Context, Result};
use clap::Parser;
use elevon_agent::{
    cli::{Cli, Commands, key::KeyCommands},
    env::ElevonEnv,
};

fn main() {
    elevon_http::init_cli_logging();

    if let Err(err) = run() {
        tracing::error!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let env = ElevonEnv::load().context("failed to load agent environment from /etc/elevon/env")?;

    match cli.command {
        Commands::Setup(args) => {
            run_async(elevon_agent::setup::setup(args.turso_remote_url))
                .context("setup command failed")?;
        }
        Commands::Api => {
            run_async(elevon_agent::api::run_api_server(&env)).context("api command failed")?;
        }
        Commands::Proxy => {
            elevon_agent::proxy::run_proxy();
        }
        Commands::InstallSystemd(args) => {
            elevon_agent::setup::install_systemd(args.enable)
                .context("failed to install systemd units")?;
        }
        Commands::Key { subcommand } => {
            run_async(KeyCommands::run(&subcommand, &env)).context("key command failed")?;
        }
    }

    Ok(())
}

fn run_async<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
