use clap::Parser;
use elevon_agent::cli::{Cli, Commands};

fn main() {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup(args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            if let Err(err) = rt.block_on(elevon_agent::setup::setup(args.turso_remote_url)) {
                tracing::error!("{err:#}");
                std::process::exit(1);
            }
        }
        Commands::Api => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(elevon_agent::api::run_api_server());
        }
        Commands::Proxy => {
            elevon_agent::proxy::run_proxy();
        }
        Commands::InstallSystemd(args) => {
            if let Err(err) = elevon_agent::setup::install_systemd(args.enable) {
                tracing::error!("{err:#}");
                std::process::exit(1);
            }
        }
    }
}
