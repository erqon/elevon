use std::process::Command;

use anyhow::{Context, Result, bail};
use elevon_fs::agent::{install_api_unit, install_proxy_unit};

use crate::db::models::AuthKey;

pub async fn setup(remote_url: Option<String>) -> Result<()> {
    tracing::info!("Setting up Elevon Agent...");
    let mut db = crate::db::init_db(remote_url).await?;

    let api_key = elevon_http::token::opaque();
    let hashed_api_key = elevon_http::token::hash(&api_key);

    toasty::create!(AuthKey {
        api_key: hashed_api_key
    })
    .exec(&mut db)
    .await?;

    tracing::info!("Your API Key: {}", &api_key);

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
