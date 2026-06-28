use toasty_cli::{Config as ToastyCliConfig, ToastyCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_config = aileron_config::load()?;
    let db = aileron_server::db::get_db(app_config).await?;

    let toasty_config = ToastyCliConfig::load()?;
    let cli = ToastyCli::with_config(db, toasty_config);
    cli.parse_and_run().await?;

    Ok(())
}
