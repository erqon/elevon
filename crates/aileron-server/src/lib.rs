mod app;
pub mod cli;
pub mod db;
mod dto;
mod routes;
mod services;

pub async fn run(config: aileron_config::Config) -> anyhow::Result<()> {
    app::serve(config).await
}
