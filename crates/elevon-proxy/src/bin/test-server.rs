use axum::{Router, routing::get};
use clap::Parser;

use elevon_auth::http::init_logging;

#[derive(Parser)]
struct Cli {
    #[arg(long, short, default_value_t = 3000)]
    port: usize,
}

#[tokio::main]
async fn main() {
    init_logging();

    let cli = Cli::parse();

    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", cli.port))
        .await
        .unwrap();
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    tracing::info!("Endpoint got hit!");

    "Hello, World!"
}
