use clap::Parser;
use elevon_deploy::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    elevon_http::init_cli_logging();

    let cli = Cli::parse();
    elevon_deploy::run_deploy_cli(cli.args, cli.command).await
}
