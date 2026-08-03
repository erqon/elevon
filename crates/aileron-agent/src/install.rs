use std::process::Command;

use aileron_fs::agent::{install_api_unit, install_proxy_unit};
use anyhow::{Context, Result, bail};

pub fn install_systemd(enable: bool) -> Result<()> {
    ensure_systemd_writable()?;

    let agent = std::env::current_exe().context("failed to resolve current executable")?;

    tracing::info!("Creating systemd units in /etc/systemd/system/ ...");

    install_api_unit(&agent).context("failed to write aileron-agent-api.service")?;
    install_proxy_unit(&agent).context("failed to write aileron-agent-proxy.service")?;

    tracing::info!("Systemd units written");

    run_systemctl(&["daemon-reload"])?;

    if enable {
        run_systemctl(&[
            "enable",
            "--now",
            "aileron-agent-api.service",
            "aileron-agent-proxy.service",
        ])?;
        tracing::info!("Services enabled and started");
    } else {
        tracing::info!(
            "Run `systemctl enable --now aileron-agent-api aileron-agent-proxy` when ready"
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
