use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

pub fn run_init() -> Result<()> {
    let config_dir = PathBuf::from(".elevon");

    if config_dir.exists() {
        tracing::warn!(".elevon directory already exists, exiting.");
        return Ok(());
    }

    fs::create_dir(&config_dir)?;

    let config_path = config_dir.join("deploy.yml");
    let config_content = include_str!("../../templates/config.yml");

    fs::write(config_path, config_content)?;

    tracing::info!("Config file was created at .elevon/deploy.yml");

    Ok(())
}

pub async fn run_upgrade(cli_version: Option<&str>, version: Option<String>) -> Result<()> {
    let current_version = cli_version.unwrap_or(env!("CARGO_PKG_VERSION"));
    let version = version.unwrap_or("latest".to_string());

    tracing::info!("Current Elevon CLI version: {}", current_version);
    tracing::info!("Looking for {} version", version);

    let client = elevon_fs::upgrade::ReleaseClient::new("elevon-cli".to_string(), version)?;
    let destination = std::env::current_exe().context("failed to resolve current executable")?;

    let installed = client.install_binary(current_version, &destination).await?;

    if installed {
        tracing::info!(path = %destination.display(), "Cli binary updated");
    }

    Ok(())
}
