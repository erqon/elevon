use std::{fs, path::PathBuf};

pub fn run() -> anyhow::Result<()> {
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
