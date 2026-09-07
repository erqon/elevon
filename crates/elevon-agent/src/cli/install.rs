use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use elevon_config::ElevonConfig;
use elevon_fs::agent::{install_api_unit, install_proxy_unit};

use crate::{
    api::db::{AgentDb, models::AuthKey},
    cli::{CliArgs, InstallArgs},
    config::Config,
    env::ElevonEnv,
};

fn check_before_installation() -> Result<()> {
    let docker_version_output = Command::new("docker")
        .arg("--version")
        .output()
        .context("failed to execute `docker --version`")?;

    if !docker_version_output.status.success() {
        bail!(
            "`docker --version` failed with {}",
            docker_version_output.status
        );
    }

    Ok(())
}

fn create_elevon_agent_user() -> Result<()> {
    let user_exists = Command::new("getent")
        .args(["passwd", "elevon-agent"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to execute `getent passwd elevon-agent`")?
        .success();

    if user_exists {
        return Ok(());
    }

    let status = Command::new("useradd")
        .args([
            "--system",
            "--home-dir",
            "/var/lib/elevon-agent",
            "--no-create-home",
            "--shell",
            "/usr/sbin/nologin",
            "elevon-agent",
        ])
        .status()
        .context("failed to execute `useradd elevon-agent`")?;

    if !status.success() {
        bail!("`useradd elevon-agent` failed with {status}");
    }

    Ok(())
}

fn ensure_agent_state_ownership() -> Result<()> {
    let status = Command::new("chown")
        .args(["-R", "elevon-agent:elevon-agent", "/var/lib/elevon-agent"])
        .status()
        .context("failed to assign ownership of /var/lib/elevon-agent")?;

    if !status.success() {
        bail!("failed to assign ownership of /var/lib/elevon-agent: {status}");
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

// create elevon-agent user
// initialize config/database/files
// chown /var/lib/elevon-agent to elevon-agent
// write units
// daemon-reload
// start services
pub async fn run(cli_args: CliArgs, args: InstallArgs) -> Result<()> {
    if !args.no_systemd {
        check_before_installation()?;
        create_elevon_agent_user()?;
    }

    let config = Config::from_file(cli_args.config).context("failed to load config")?;
    config.setup_agent()?;

    ElevonEnv::new(config.agent.domain, config.turso_remote_url)
        .context("failed to init agent env")?;

    let mut agent_db = AgentDb::new(None)
        .await
        .context("failed to initialize the agent database")?;

    ensure_agent_state_ownership()?;

    let auth_keys = AuthKey::all().exec(&mut agent_db.db).await?;

    if !auth_keys.is_empty() {
        tracing::info!("Auth key already exist, run 'key list' to view the keys");
    } else {
        crate::cli::key::create("Default").await?;
    }

    if !args.no_systemd {
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
    }

    Ok(())
}
