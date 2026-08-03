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
    #[command(about = "Run the reverse proxy for the agent.")]
    Proxy,

    #[command(about = "Run the agent HTTP control API")]
    Api,

    #[command(about = "Install systemd units for the server and proxy")]
    InstallSystemd(InstallSystemdArgs),
}

#[derive(Args)]
pub struct InstallSystemdArgs {
    /// Run systemctl daemon-reload && enable --now after writing units
    #[arg(long)]
    pub enable: bool,
}
