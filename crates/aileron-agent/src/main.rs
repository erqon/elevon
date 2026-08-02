use aileron_agent::cli::{Cli, Commands};
use clap::Parser;

fn main() {
    aileron_auth::http::init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(aileron_agent::server::run_server());
        }
        Commands::Proxy => {
            aileron_agent::proxy::run_proxy();
        }
    }
}
