use clap::{
    Parser,
    builder::styling::{AnsiColor, Styles},
};

use crate::commands::Commands;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(
    name = "elevon",
    version,
    about,
    long_about = "Elevon is a zero-SSH, agent-driven tool for deploying and managing containers on remote VMs.",
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
