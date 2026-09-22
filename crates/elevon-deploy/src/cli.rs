pub mod init;

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
    name = "elevon-deploy",
    version,
    about,
    long_about = "Elevon Deploy: build and push container images for Elevon apps.",
    styles = STYLES
)]
pub struct Cli {
    #[command(flatten)]
    pub args: DeployArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Args, Clone)]
pub struct DeployArgs {
    #[arg(
        short,
        long,
        conflicts_with = "config",
        help = "Project name used to find deploy.<name>.yml"
    )]
    pub project: Option<String>,

    #[arg(
        short,
        long,
        conflicts_with = "project",
        help = "Path to the deploy configuration file (defaults to .elevon/deploy.yml)"
    )]
    pub config: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a configuration file template")]
    Init,

    #[command(about = "Checks for deployment needed assets")]
    Check,

    #[command(about = "Upgrade the CLI binary")]
    Upgrade(UpgradeArgs),

    #[command(about = "Build container images from the deploy config")]
    Build(BuildArgs),

    #[command(about = "Push configured images to the registry")]
    Push,

    #[command(about = "Deploy applications")]
    Deploy(AppArgs),

    #[command(about = "Rollback applications to previous deployment")]
    Rollback(AppArgs),
}

#[derive(Debug, Args)]
pub struct UpgradeArgs {
    #[arg(long, help = "The version to upgrade to")]
    pub version: Option<String>,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long, short, help = "Push the image to the registry")]
    pub push: bool,
}

#[derive(Debug, Args)]
pub struct AppArgs {
    #[arg(long = "app", help = "Limit the operation to the specified app(s)")]
    pub apps: Vec<String>,
}
