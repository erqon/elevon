use std::process::Command;

use anyhow::{Context, Result, bail};
use elevon_fs::agent::{install_api_unit, install_proxy_unit};

use crate::{
    api::db::{AgentDb, models::AuthKey},
    cli::InstallSystemdArgs,
    env::{ElevonEnv, ElevonEnvKey},
};

pub async fn setup(turso_remote_url: Option<String>) -> Result<()> {
    tracing::info!("Setting up Elevon Agent...");

    let mut env = ElevonEnv::load().context("failed to load agent env before setup")?;

    if let Some(remote_url) = turso_remote_url {
        env.update_value(ElevonEnvKey::TursoRemoteUrl, remote_url)
            .context("failed to persist the Turso remote URL")?;
    }

    let mut agent_db = AgentDb::new(env.turso_remote_url.as_deref())
        .await
        .context("failed to initialize the agent database")?;

    let auth_keys = AuthKey::all().exec(&mut agent_db.db).await?;

    if !auth_keys.is_empty() {
        tracing::info!("Auth key already exist, run 'agent key list' to view the keys");
    } else {
        crate::cli::key::create("Default", env.turso_remote_url).await?;
    }

    tracing::info!("Elevon Agent has been setup. Now you can run ---");

    Ok(())
}

pub fn install_systemd(args: InstallSystemdArgs) -> Result<()> {
    ensure_systemd_writable()?;

    let agent = std::env::current_exe().context("failed to resolve current executable")?;

    tracing::info!("Creating systemd units in /etc/systemd/system/ ...");

    install_api_unit(&agent).context("failed to write elevon-agent-api.service")?;
    install_proxy_unit(
        &agent,
        &args.proxy_args.agent_domain,
        args.proxy_args.tls_cert_path.as_deref(),
        args.proxy_args.tls_cert_path.as_deref(),
    )
    .context("failed to write elevon-agent-proxy.service")?;

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
