use clap::Parser;
use elevon_cli::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();

    if let Some(command) = cli.deploy_command {
        let args = cli.deploy_args.unwrap_or_default();
        elevon_deploy::run_deploy_cli(args, command).await?;
    }

    Ok(())
}
