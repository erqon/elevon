use clap::Parser;
use elevon_cli::cli::Cli;

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();
}
