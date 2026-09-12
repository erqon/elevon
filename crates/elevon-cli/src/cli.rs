use clap::{
    Parser,
    builder::styling::{AnsiColor, Styles},
};

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
    styles = STYLES,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(flatten)]
    pub deploy_args: elevon_deploy::cli::DeployArgs,

    #[command(subcommand)]
    pub deploy_command: Option<elevon_deploy::cli::Commands>,
}
