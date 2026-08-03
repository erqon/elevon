use elevon_config::ElevonConfig;
use elevon_server::Config;
use tracing_subscriber::{EnvFilter, fmt};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));

    match std::env::var("LOG_FORMAT").as_deref() {
        Ok("json") => {
            fmt()
                .json()
                .flatten_event(true)
                .with_span_list(false)
                .with_env_filter(filter)
                .init();
        }
        _ => {
            fmt().pretty().with_env_filter(filter).init();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let config = Config::from_file("elevon.yml")?;

    elevon_server::run(config).await
}
