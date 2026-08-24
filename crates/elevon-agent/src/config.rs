use elevon_contracts::deploy::TlsConfig;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AgentConfig {
    pub domain: String,
    pub tls: TlsConfig,
}

#[derive(Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
}
