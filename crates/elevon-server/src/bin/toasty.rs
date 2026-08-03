use elevon_config::ElevonConfig;
use elevon_server::Config;
use toasty_cli::{Config as ToastyCliConfig, ToastyCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_config = Config::from_file("elevon.yml")?;
    let db = elevon_server::db::get_db(&app_config.database).await?;

    let toasty_config = ToastyCliConfig::load()?;
    let cli = ToastyCli::with_config(db, toasty_config);
    cli.parse_and_run().await?;

    Ok(())
}
