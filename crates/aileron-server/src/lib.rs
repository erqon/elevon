mod app;
pub mod config;
pub mod db;
mod payload;
mod route;
pub mod service;

pub use config::*;

pub async fn run(config: Config) -> anyhow::Result<()> {
    app::serve(config).await
}
