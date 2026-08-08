pub mod key;

use clap::{
    Args, Parser, Subcommand,
    builder::styling::{AnsiColor, Styles},
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(
    name = "elevon-agent",
    version,
    about,
    long_about = "elevon Agent: A zero-SSH, agent-driven container orchestrator.",
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Setup agent in your vps")]
    Setup(SetupArgs),

    #[command(about = "Run the reverse proxy for the agent.")]
    Proxy,

    #[command(about = "Run the agent HTTP control API")]
    Api,

    #[command(about = "Install systemd units for the server and proxy")]
    InstallSystemd(InstallSystemdArgs),

    #[command(about = "Manage auth keys")]
    Key {
        #[command(subcommand)]
        subcommand: key::KeyCommands,
    },
}

#[derive(Args)]
pub struct InstallSystemdArgs {
    #[arg(
        long,
        help = "Run systemctl daemon-reload && enable --now after writing units"
    )]
    pub enable: bool,
}

#[derive(Args)]
pub struct SetupArgs {
    #[arg(
        long,
        help = "Optional Turso remote URL for syncing local data to the cloud"
    )]
    pub turso_remote_url: Option<String>,
}
