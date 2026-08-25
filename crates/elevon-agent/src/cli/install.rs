use std::process::Command;

use anyhow::{Context, Result, bail};
use elevon_config::ElevonConfig;
use elevon_fs::agent::{install_api_unit, install_proxy_unit};

use crate::{
    api::db::{AgentDb, models::AuthKey},
    cli::{CliArgs, InstallArgs},
    config::Config,
    env::ElevonEnv,
};

fn ensure_systemd_writable() -> Result<()> {
    let dir = "/etc/systemd/system";
    let metadata =
        std::fs::metadata(dir).with_context(|| format!("systemd dir not found: {dir}"))?;

    if !metadata.is_dir() {
        bail!("{dir} is not a directory");
    }

    let probe = std::path::Path::new(dir).join(".elevon-write-test");
    std::fs::write(&probe, b"").with_context(|| format!("permission denied writing to {dir}"))?;
    std::fs::remove_file(&probe)?;

    Ok(())
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .args(args)
        .status()
        .with_context(|| format!("failed to run systemctl {}", args.join(" ")))?;

    if !status.success() {
        bail!("systemctl {} failed with {status}", args.join(" "));
    }
    Ok(())
}

pub async fn run(cli_args: CliArgs, args: InstallArgs) -> Result<()> {
    ensure_systemd_writable()?;

    let config = Config::from_file(cli_args.config).context("failed to load config")?;
    config.setup_agent()?;

    ElevonEnv::new(config.agent.domain, config.turso_remote_url)
        .context("failed to init agent env")?;

    let mut agent_db = AgentDb::new(None)
        .await
        .context("failed to initialize the agent database")?;

    let auth_keys = AuthKey::all().exec(&mut agent_db.db).await?;

    if !auth_keys.is_empty() {
        tracing::info!("Auth key already exist, run 'key list' to view the keys");
    } else {
        let api_key = crate::cli::key::create("Default").await?;

        tracing::info!("API Key was created, make sure to save it: {}", &api_key);
    }

    let agent = std::env::current_exe().context("failed to resolve current executable")?;

    tracing::info!("Creating systemd units in /etc/systemd/system/ ...");

    install_api_unit(&agent).context("failed to write elevon-agent-api.service")?;
    install_proxy_unit(&agent).context("failed to write elevon-agent-proxy.service")?;

    tracing::info!("Systemd units written");

    run_systemctl(&["daemon-reload"])?;

    if args.enable {
        run_systemctl(&[
            "enable",
            "--now",
            "elevon-agent-api.service",
            "elevon-agent-proxy.service",
        ])?;
        tracing::info!("Services enabled and started");
    } else {
        tracing::info!(
            "Run `systemctl enable --now elevon-agent-api elevon-agent-proxy` when ready"
        );
    }

    Ok(())
}
