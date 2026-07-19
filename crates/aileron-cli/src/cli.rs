use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Styles},
};

use crate::commands::{DashboardCommands, server::ServerCommands};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(
    name = "aileron",
    version,
    about,
    long_about = "Aileron: A zero-SSH, agent-driven container orchestrator and Pingora-powered edge proxy featuring a unified dashboard to monitor, manage, and load-balance server clusters with zero downtime.",
    styles = STYLES
)]
pub struct Cli {
    #[arg(
        short,
        long,
        default_value = "aileron.yml",
        help = "Path to the config file"
    )]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand, version = "0.1.0")]
    Dashboard(DashboardCommands),

    #[command(subcommand, name = "serve", version = "0.1.0")]
    Server(ServerCommands),
}
