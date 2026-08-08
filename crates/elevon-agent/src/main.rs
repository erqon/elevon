use clap::Parser;
use elevon_agent::cli::{Cli, Commands, key::KeyCommands};

fn main() -> anyhow::Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup(args) => {
            run_async(elevon_agent::setup::setup(args.turso_remote_url))?;
        }
        Commands::Api => {
            run_async(elevon_agent::api::run_api_server());
        }
        Commands::Proxy => {
            elevon_agent::proxy::run_proxy();
        }
        Commands::InstallSystemd(args) => {
            elevon_agent::setup::install_systemd(args.enable)?;
        }
        Commands::Key { subcommand } => {
            run_async(KeyCommands::run(&subcommand));
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
