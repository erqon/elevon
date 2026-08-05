use clap::Parser;
use elevon_config::{ElevonConfig, ResolveEnvCredentials};
use elevon_deploy::cli::{Cli, Commands};
use elevon_deploy::config::Config;
use elevon_deploy::config::app::AppConfig;
use tracing_indicatif::IndicatifLayer;
use tracing_indicatif::filter::IndicatifFilter;
use tracing_indicatif::style::ProgressStyle;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn init_logging() {
    let indicatif_layer = IndicatifLayer::new().with_progress_style(
        ProgressStyle::with_template(
            "{span_child_prefix}{spinner:.cyan} {span_name:.bold.cyan} {msg:.dim}",
        )
        .expect("progress style template")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );

    // spinner for instrumented spans; docker steps are plain prints
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false)
                .with_level(true)
                .with_ansi(true)
                .with_writer(indicatif_layer.get_stderr_writer()),
        )
        .with(indicatif_layer.with_filter(IndicatifFilter::new(true)))
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();

    let cli = Cli::parse();
    let config = Config::from_file(&cli.config)?;

    match cli.command {
        Commands::Build(args) => {
            let apps = config.apps()?;

            let selected: Vec<(&String, &AppConfig)> = if args.apps.is_empty() {
                apps.iter().map(|(n, a)| (n, *a)).collect()
            } else {
                let mut out = Vec::with_capacity(args.apps.len());
                for name in &args.apps {
                    let app = apps
                        .get(name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("unknown app `{name}`"))?;
                    out.push((name, app));
                }
                out
            };

            for (name, app) in selected {
                let image = app
                    .image
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("app `{name}` is missing `image`"))?
                    .to_owned();
                let build = app
                    .build
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("app `{name}` is missing `build`"))?
                    .clone();

                elevon_deploy::image::build_image(
                    &image,
                    &config.registry.server,
                    &build,
                    &cli.config,
                )
                .await?;
            }
        }
        Commands::Push => {
            let app = config
                .app_config
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing root app config"))?;

            let image = app
                .image
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing image"))?;

            let registry_credentials = config.registry.resolved_credentials()?;

            elevon_deploy::image::push_image(image, registry_credentials).await?;
        }
    }

    Ok(())
}
