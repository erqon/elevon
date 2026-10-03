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
    Stop(RemoveAppCommandArgs),
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum DeploymentCommands {
    #[command(about = "List deployments")]
    List(ListQueryParams),
}

#[derive(Args, Serialize, Deserialize)]
pub struct RemoveAppCommandArgs {
    #[arg(
        value_name = "APP",
        required = true,
        help = "An app ID/name or list of app IDs/names to remove"
    )]
    pub apps: Vec<String>,

    #[arg(short, long, help = "Forces removal when the app is running")]
    pub force: bool,
}
