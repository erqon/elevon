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

#[derive(Deserialize)]
pub struct ProxyConfig {
    pub port: Option<u16>,
}

#[derive(Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub proxy: Option<ProxyConfig>,
    pub turso_remote_url: Option<String>,
}

impl Config {
    pub fn setup(&self, agent_name: &str) -> anyhow::Result<()> {
        if let Some(tls) = &self.agent.tls {
            let cert_bytes = std::fs::read(tls.cert.clone())?;
            let key_bytes = std::fs::read(tls.key.clone())?;

            let options = TlsOptions {
                project: "agent".to_string(),
                app: None,
            };

            write_tls_file(agent_name, &cert_bytes, TlsType::Cert, options.clone())?;
            write_tls_file(agent_name, &key_bytes, TlsType::Key, options)?;
        }

        Ok(())
    }
}

impl ElevonConfig for Config {}
