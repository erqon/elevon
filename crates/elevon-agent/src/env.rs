use std::collections::HashMap;

use anyhow::Result;
use elevon_fs::agent::{AppEnvOptions, add_app_env, load_app_env};

use crate::config::Config;

#[derive(Debug, Clone, Default)]
pub struct ElevonEnv {
    pub agent_domain: String,
    pub agent_port: u16,
    pub proxy_port: Option<u16>,
    pub turso_remote_url: Option<String>,
}

impl ElevonEnv {
    pub fn new(agent_name: &str, config: Config) -> Result<Self> {
        let mut elevon_env = Self {
            agent_domain: config.agent.domain.clone(),
            agent_port: config.agent.port,
            proxy_port: config.proxy.and_then(|c| c.port),
            turso_remote_url: config.turso_remote_url,
        };
        elevon_env.set_env_values(agent_name)?;
        Ok(elevon_env)
    }

    pub fn load(agent_name: &str) -> Result<Self> {
        let vars = load_app_env(agent_name, AppEnvOptions::elevon())?;
        let mut env = Self::default();

        for key in ElevonEnvKey::all() {
            if let Some(value) = key.read_from(&vars) {
                env.apply_value(key, value)?;
            }
        }

        Ok(env)
    }

    // updates the in-memory field only, without touching disk
    fn apply_value(&mut self, key: ElevonEnvKey, value: String) -> Result<()> {
        match key {
            ElevonEnvKey::AgentDomain => self.agent_domain = value,
            ElevonEnvKey::AgentPort => self.agent_port = value.parse::<u16>()?,
            ElevonEnvKey::ProxyPort => self.proxy_port = Some(value.parse::<u16>()?),
            ElevonEnvKey::TursoRemoteUrl => self.turso_remote_url = Some(value),
        }

        Ok(())
    }

    fn set_value(&mut self, agent_name: &str, key: ElevonEnvKey, value: String) -> Result<()> {
        let persisted = value.clone();

        self.apply_value(key, value)?;

        add_app_env(
            agent_name,
            key.bare_name().to_string(),
            persisted,
            AppEnvOptions::elevon(),
        )?;

        Ok(())
    }

    fn set_env_values(&mut self, agent_name: &str) -> Result<()> {
        self.set_value(
            agent_name,
            ElevonEnvKey::AgentDomain,
            self.agent_domain.clone(),
        )?;
        self.set_value(
            agent_name,
            ElevonEnvKey::AgentDomain,
            self.agent_port.to_string(),
        )?;

        if let Some(proxy_port) = self.proxy_port {
            self.set_value(agent_name, ElevonEnvKey::ProxyPort, proxy_port.to_string())?;
        }

        if let Some(turso_remote_url) = &self.turso_remote_url {
            self.set_value(
                agent_name,
                ElevonEnvKey::TursoRemoteUrl,
                turso_remote_url.clone(),
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ElevonEnvKey {
    AgentDomain,
    AgentPort,
    ProxyPort,
    TursoRemoteUrl,
}

impl ElevonEnvKey {
    pub fn all() -> [Self; 2] {
        [Self::AgentDomain, Self::TursoRemoteUrl]
    }

    pub fn bare_name(&self) -> &'static str {
        match self {
            Self::AgentDomain => "AGENT_DOMAIN",
            Self::AgentPort => "AGENT_PORT",
            Self::ProxyPort => "PROXY_PORT",
            Self::TursoRemoteUrl => "TURSO_REMOTE_URL",
        }
    }

    fn read_from(&self, vars: &HashMap<String, String>) -> Option<String> {
        vars.get(&self.bare_name().to_string()).cloned()
    }

    pub fn from_bare_name(s: &str) -> Option<Self> {
        Self::all().into_iter().find(|k| k.bare_name() == s)
    }
}

impl std::str::FromStr for ElevonEnvKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bare_name(s).ok_or_else(|| anyhow::anyhow!("unknown env key: {s}"))
    }
}
