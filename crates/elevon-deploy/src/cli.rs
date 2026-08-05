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
    long_about = "Elevon Deploy: A container orchestrator, working alongside with Elevon Agent.",
    styles = STYLES
)]
pub struct Cli {
    #[arg(
        short,
        long,
        default_value = ".elevon/deploy.yml",
        help = "Path to the config file"
    )]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Build container images from the deploy config")]
    Build(BuildArgs),

    #[command(about = "Push already built image to the registry")]
    Push,
}

#[derive(Args)]
pub struct BuildArgs {
    #[arg(long, help = "List of apps you want to build")]
    pub app: Option<Vec<String>>,
}
