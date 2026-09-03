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
        default_value = ".elevon/deploy.yml",
        help = "Path to the deploy configuration file"
    )]
    pub config: String,
}

impl Default for DeployArgs {
    fn default() -> Self {
        Self {
            config: ".elevon/deploy.yml".to_string(),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a configuration file template")]
    Init,

    #[command(about = "Checks for deployment needed assets")]
    Check,

    #[command(about = "Build container images from the deploy config")]
    Build(BuildArgs),

    #[command(about = "Push configured images to the registry")]
    Push,

    #[command(about = "Deploy applications")]
    Deploy(AppArgs),

    #[command(about = "Rollback applications to previous deployment")]
    Rollback(RollbackArgs),
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

    #[arg(
        long,
        default_value_t = false,
        help = "Build and push the image before deploying"
    )]
    pub build: bool,
}

#[derive(Debug, Args)]
pub struct RollbackArgs {
    #[arg(long = "app", help = "Limit the operation to the specified app(s)")]
    pub apps: Vec<String>,
}
