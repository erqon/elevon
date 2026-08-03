use elevon_agent::cli::{Cli, Commands};
use clap::Parser;

fn main() {
    elevon_auth::http::init_logging();

    let cli = Cli::parse();

    match cli.command {
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
            if let Err(err) = elevon_agent::install::install_systemd(args.enable) {
                tracing::error!("{err:#}");
                std::process::exit(1);
            }
        }
    }
}
