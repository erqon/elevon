use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Styles},
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(
    name = "aileron-agent",
    version,
    about,
    long_about = "Aileron Agent: A zero-SSH, agent-driven container orchestrator.",
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run the agent as a daemon")]
    Serve,

    #[command(about = "Run the reverse proxy for the agent.")]
    Proxy,
}
