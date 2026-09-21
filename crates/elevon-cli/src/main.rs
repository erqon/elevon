use clap::Parser;
use elevon_cli::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.deploy_command {
        elevon_deploy::run_deploy_cli(cli.deploy_args, command).await?;
    }

    Ok(())
}
