use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::cli::ListQueryParams;

#[derive(Subcommand, Serialize, Deserialize)]
pub enum AppCommands {
    #[command(about = "List apps")]
    List(ListQueryParams),

    #[command(about = "Deployment commands")]
    Deployment {
        #[command(subcommand)]
        subcommand: DeploymentCommands,
    },

    #[command(about = "Stop application")]
    Stop(AppStopCommandArgs),
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum DeploymentCommands {
    #[command(about = "List deployments")]
    List(ListQueryParams),
}

#[derive(Args, Serialize, Deserialize)]
pub struct AppStopCommandArgs {
    #[arg(
        value_name = "APP",
        required = true,
        help = "App ID or <project-name:app-name> (or a list of them) to stop."
    )]
    pub apps: Vec<String>,

    #[arg(short, long, help = "Forces stop when the app is running")]
    pub force: bool,
}
