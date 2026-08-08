use std::process::Command;

use anyhow::{Context, Result, bail};
use elevon_fs::agent::{install_api_unit, install_proxy_unit};

use crate::{
    db::models::AuthKey,
    env::{ElevonEnv, ElevonEnvKey},
};

pub async fn setup(turso_remote_url: Option<String>) -> Result<()> {
    tracing::info!("Setting up Elevon Agent...");

    let mut env = ElevonEnv::load()?;

    if let Some(remote_url) = turso_remote_url {
        env.update_value(ElevonEnvKey::TursoRemoteUrl, remote_url)?;
    }

    let mut db = crate::db::init_db(env.turso_remote_url.as_deref()).await?;

    let auth_keys = AuthKey::all().exec(&mut db).await?;

    if auth_keys.len() > 0 {
        tracing::info!("Auth key already exist, run agent key list to view the keys");
    } else {
        crate::cli::key::create("Default", env.turso_remote_url).await?;
    }

    tracing::info!("Elevon Agent has been setup. Now you can run ---");

    Ok(())
}

pub fn install_systemd(enable: bool) -> Result<()> {
    ensure_systemd_writable()?;

    let agent = std::env::current_exe().context("failed to resolve current executable")?;

    tracing::info!("Creating systemd units in /etc/systemd/system/ ...");

    install_api_unit(&agent).context("failed to write elevon-agent-api.service")?;
    install_proxy_unit(&agent).context("failed to write elevon-agent-proxy.service")?;

    tracing::info!("Systemd units written");

    run_systemctl(&["daemon-reload"])?;

    if enable {
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

fn ensure_systemd_writable() -> Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/etc/systemd/system")
        .context("permission denied writing to /etc/systemd/system (try sudo)")?;
    Ok(())
}
