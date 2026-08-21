use clap::Parser;
use elevon_cli::{cli::Cli, commands::Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy { args, command } => {
            elevon_deploy::run_deploy_cli(args, command).await?;
        }
    }

    Ok(())
}
