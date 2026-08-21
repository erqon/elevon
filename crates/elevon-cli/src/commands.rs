use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Deploy apps using the configured agent")]
    Deploy {
        #[command(flatten)]
        args: elevon_deploy::cli::DeployArgs,

        #[command(subcommand)]
        command: elevon_deploy::cli::Commands,
    },
}
