use elevon_config::ElevonConfig;
use elevon_contracts::deploy::{TlsConfig, TlsType};
use elevon_fs::agent::{TlsOptions, write_tls_file};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AgentConfig {
    pub domain: String,
    pub tls: TlsConfig,
}

#[derive(Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub turso_remote_url: Option<String>,
}

impl Config {
    pub fn setup_agent(&self) -> anyhow::Result<()> {
        let cert_bytes = std::fs::read(self.agent.tls.cert.clone())?;
        let key_bytes = std::fs::read(self.agent.tls.key.clone())?;

        let options = TlsOptions {
            project: "agent".to_string(),
            app: None,
        };

        write_tls_file(&cert_bytes, TlsType::Cert, options.clone())?;
        write_tls_file(&key_bytes, TlsType::Key, options)?;

        Ok(())
    }
}

impl ElevonConfig for Config {}
