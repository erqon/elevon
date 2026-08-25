use std::collections::HashMap;

use anyhow::Result;
use elevon_fs::agent::{add_app_env, load_app_env};

#[derive(Debug, Clone, Default)]
pub struct ElevonEnv {
    pub agent_domain: String,
    pub turso_remote_url: Option<String>,
}

impl ElevonEnv {
    pub fn new(agent_domain: String, turso_remote_url: Option<String>) -> Result<Self> {
        let vars = load_app_env("default", Some(true))?;
        let mut env = Self {
            agent_domain,
            turso_remote_url,
        };

        for key in ElevonEnvKey::all() {
            if let Some(value) = key.read_from(&vars) {
                env.set_value(key, value)?;
            }
        }

        Ok(env)
    }

    pub fn load() -> Result<Self> {
        let vars = load_app_env("default", Some(true))?;
        let mut env = Self::default();

        for key in ElevonEnvKey::all() {
            if let Some(value) = key.read_from(&vars) {
                env.apply_value(key, value);
            }
        }

        Ok(env)
    }

    // updates the in-memory field only, without touching disk
    fn apply_value(&mut self, key: ElevonEnvKey, value: String) {
        match key {
            ElevonEnvKey::AgentDomain => self.agent_domain = value,
            ElevonEnvKey::TursoRemoteUrl => self.turso_remote_url = Some(value),
        }
    }

    fn set_value(&mut self, key: ElevonEnvKey, value: String) -> Result<()> {
        let persisted = value.clone();

        self.apply_value(key, value);

        add_app_env(
            "default",
            Some(true),
            key.bare_name().to_string(),
            persisted,
        )?;

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ElevonEnvKey {
    AgentDomain,
    TursoRemoteUrl,
}

impl ElevonEnvKey {
    pub fn all() -> [Self; 2] {
        [Self::AgentDomain, Self::TursoRemoteUrl]
    }

    pub fn bare_name(&self) -> &'static str {
        match self {
            Self::AgentDomain => "AGENT_DOMAIN",
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
