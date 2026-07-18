use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "aileron.yml", help = "Path to the config file")]
    config: Option<String>
}

#[tokio::main]
async fn main() {
    let _args = Args::parse();
}
