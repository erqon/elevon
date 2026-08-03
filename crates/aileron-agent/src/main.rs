use aileron_agent::cli::{Cli, Commands};
use clap::Parser;

fn main() {
    aileron_auth::http::init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Api => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(aileron_agent::api::run_api_server());
        }
        Commands::Proxy => {
            aileron_agent::proxy::run_proxy();
        }
        Commands::InstallSystemd(args) => {
            if let Err(err) = aileron_agent::install::install_systemd(args.enable) {
                tracing::error!("{err:#}");
                std::process::exit(1);
            }
        }
    }
}
