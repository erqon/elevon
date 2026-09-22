use clap::Parser;
use elevon_deploy::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    elevon_deploy::run_deploy_cli(cli.args, cli.command, None).await
}
