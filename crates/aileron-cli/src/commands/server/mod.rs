use clap::Subcommand;

#[derive(Subcommand)]
#[command(long_about = "test")]
pub enum ServerCommands {}
