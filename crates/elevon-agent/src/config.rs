use anyhow::{Context, Result};
use elevon_config::ElevonConfig;
use elevon_contracts::deploy::{TlsConfig, TlsType};
use elevon_fs::agent::{TlsOptions, write_tls_file};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AgentConfig {
    pub domain: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

fn default_ports() -> String {
    "80:443".to_string()
}

#[derive(Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_ports")]
    pub port: String,
}

pub fn parse_port(ports: String) -> Result<(u16, Option<u16>)> {
    let mut parts = ports.split(":");

    let http = parts
        .next()
        .context("Missing HTTP port")?
        .parse::<u16>()
        .context("Invalid u16 for HTTP port")?;

    let https = parts
        .next()
        .map(|s| s.parse::<u16>().context("Invalid u16 for HTTPS port"))
        .transpose()?;

    Ok((http, https))
}

#[derive(Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub proxy: Option<ProxyConfig>,
    pub turso_remote_url: Option<String>,
}

impl Config {
    pub fn setup(&self) -> Result<()> {
        if let Some(tls) = &self.agent.tls {
            let cert_bytes = std::fs::read(tls.cert.clone())?;
            let key_bytes = std::fs::read(tls.key.clone())?;

            let options = TlsOptions {
                project: "agent".to_string(),
                app: None,
            };

            write_tls_file(&cert_bytes, TlsType::Cert, options.clone())?;
            write_tls_file(&key_bytes, TlsType::Key, options)?;
        }

        Ok(())
    }
}

impl ElevonConfig for Config {}
