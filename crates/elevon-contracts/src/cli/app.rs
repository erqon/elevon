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

    #[command(about = "Remove application")]
    Remove(RemoveCommandArgs),
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum DeploymentCommands {
    #[command(about = "List deployments")]
    List(ListQueryParams),
}

#[derive(Args, Serialize, Deserialize)]
pub struct RemoveCommandArgs {
    #[arg(long, help = "Name of the app you want to remove")]
    pub name: String,

    #[arg(short, long, help = "Forces removal when the app is running")]
    pub force: bool,
}
