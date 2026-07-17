mod app;
pub mod db;
mod payload;
mod route;
pub mod service;

pub async fn run(config: aileron_config::Config) -> anyhow::Result<()> {
    app::serve(config).await
}
