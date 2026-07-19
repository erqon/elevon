use clap::{
    Parser,
    builder::styling::{AnsiColor, Styles},
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser, Debug)]
#[command(
    name = "aileron",
    version,
    about,
    long_about = "Aileron: A zero-SSH, agent-driven container orchestrator and Pingora-powered edge proxy featuring a unified dashboard to monitor, manage, and load-balance server clusters with zero downtime.",
    styles = STYLES
)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "aileron.yml",
        help = "Path to the config file"
    )]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let _args = Args::parse();
}
