use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Serialize, Deserialize)]
pub enum AppCommands {
    #[command(about = "List all apps")]
    List,
}
