mod app;
pub mod cli;
pub mod db;
mod payload;
mod route;
mod service;

pub async fn run(config: aileron_config::Config) -> anyhow::Result<()> {
    app::serve(config).await
}
