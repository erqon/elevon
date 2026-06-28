mod app;
pub mod db;
mod routes;

pub async fn run(config: aileron_config::Config) -> anyhow::Result<()> {
    app::serve(config).await
}
