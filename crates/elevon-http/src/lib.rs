pub mod auth;
pub mod error;
pub mod runtime;
pub mod token;

use tracing_indicatif::filter::IndicatifFilter;
use tracing_indicatif::{IndicatifLayer, style::ProgressStyle};
use tracing_subscriber::fmt::time::SystemTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn check_port(port: u16) -> Option<u16> {
    match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => Some(port),
        Err(_) => None,
    }
}

pub fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=info"));

    match std::env::var("LOG_FORMAT").as_deref() {
        Ok("json") => {
            tracing_subscriber::fmt()
                .json()
                .flatten_event(true)
                .with_span_list(false)
                .with_env_filter(filter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .pretty()
                .with_env_filter(filter)
                .init();
        }
    }
}

pub fn init_cli_logging(with_level: bool) {
    let indicatif_layer = IndicatifLayer::new().with_progress_style(
        ProgressStyle::with_template(
            "{span_child_prefix}{spinner:.cyan} {span_name:.bold.cyan} {msg:.dim}",
        )
        .expect("progress style template")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "warn,elevon_agent=info,elevon_deploy=info,elevon_contracts=info,turso_sync_engine=warn,toasty=warn",
        )
    });

    let formatter = if with_level {
        tracing_subscriber::fmt::layer()
            .with_timer(SystemTime)
            .with_target(false)
            .with_level(true)
            .with_ansi(true)
            .with_writer(indicatif_layer.get_stderr_writer())
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .without_time()
            .with_target(false)
            .with_level(false)
            .with_ansi(true)
            .with_writer(indicatif_layer.get_stderr_writer())
            .boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(formatter)
        .with(indicatif_layer.with_filter(IndicatifFilter::new(true)))
        .init();
}
