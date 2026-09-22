pub mod init;
pub mod install;
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
    #[command(flatten)]
    pub args: CliArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct CliArgs {
    #[arg(
        short,
        long,
        global = true,
        value_name = "PATH",
        default_value = "elevon-agent.yml",
        help = "Path to the config file"
    )]
    pub config: String,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Init config file")]
    Init,

    #[command(about = "Install agent as a systemd service")]
    Install(InstallArgs),

    #[command(about = "Uninstall agent")]
    Uninstall(UninstallArgs),

    #[command(about = "Upgrade to new version")]
    Upgrade(UpgradeArgs),

    #[command(about = "Run the reverse proxy for the agent")]
    Proxy(ProxyArgs),

    #[command(about = "Run the agent HTTP control API")]
    Api(ApiArgs),

    #[command(about = "Manage auth keys")]
    Key {
        #[command(subcommand)]
        subcommand: key::KeyCommands,
    },
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    #[arg(
        long,
        help = "Run systemctl daemon-reload && enable --now after writing units"
    )]
    pub enable: bool,

    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        default_value_t = false,
        help = "Avoid creating systemd units"
    )]
    pub no_systemd: bool,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    #[arg(long, short = 'y')]
    pub yes: bool,

    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        default_value_t = false,
        help = "Keep systemd units"
    )]
    pub keep_systemd: bool,

    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        default_value_t = false,
        help = "Keep the database"
    )]
    pub keep_database: bool,

    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        default_value_t = false,
        help = "Keep the environment files"
    )]
    pub keep_env_files: bool,
}

#[derive(Args)]
pub struct UpgradeArgs {
    #[arg(long, help = "The version to upgrade to")]
    pub version: Option<String>,

    #[arg(long,
        action = clap::ArgAction::SetTrue,
        default_value_t = false,
        help = "Reload running services"
    )]
    pub reload_services: bool,
}

#[derive(Args)]
pub struct ProxyArgs {
    #[arg(
        short,
        long,
        value_name = "PORT",
        help = "Custom ports run HTTP only. Use a tunnel or reverse proxy for public HTTPS."
    )]
    pub port: Option<u16>,
}

#[derive(Args)]
pub struct ApiArgs {
    #[arg(
        short,
        long,
        value_name = "PORT",
        default_value_t = 3333,
        help = "Listen on PORT"
    )]
    pub port: u16,
}
