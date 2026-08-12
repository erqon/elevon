use std::collections::HashMap;

use anyhow::Result;
use elevon_fs::agent::{add_app_env, load_app_env};

#[derive(Debug, Clone, Default)]
pub struct ElevonEnv {
    pub turso_remote_url: Option<String>,
    pub registry_server: String,
    pub registry_username: String,
    pub registry_password: String,
}

impl ElevonEnv {
    pub fn load() -> Result<Self> {
        let vars = load_app_env("default", Some(true))?;
        let mut env = Self::default();

        for key in ElevonEnvKey::all() {
            if let Some(value) = key.read_from(&vars) {
                env.set_value(key, value)?;
            }
        }

        Ok(env)
    }

    pub fn update_value(&mut self, key: ElevonEnvKey, value: impl Into<String>) -> Result<()> {
        self.set_value(key, value.into())?;
        Ok(())
    }

    fn set_value(&mut self, key: ElevonEnvKey, value: String) -> Result<()> {
        let persisted = value.clone();

        match key {
            ElevonEnvKey::TursoRemoteUrl => {
                self.turso_remote_url = Some(value);
            }
            ElevonEnvKey::RegistryServer => {
                self.registry_server = value;
            }
            ElevonEnvKey::RegistryUsername => {
                self.registry_username = value;
            }
            ElevonEnvKey::RegistryPassword => {
                self.registry_password = value;
            }
        }

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
    TursoRemoteUrl,
    RegistryServer,
    RegistryUsername,
    RegistryPassword,
}

impl ElevonEnvKey {
    pub fn all() -> [Self; 4] {
        [
            Self::TursoRemoteUrl,
            Self::RegistryServer,
            Self::RegistryUsername,
            Self::RegistryPassword,
        ]
    }

    pub fn bare_name(&self) -> &'static str {
        match self {
            Self::TursoRemoteUrl => "TURSO_REMOTE_URL",
            Self::RegistryServer => "REGISTRY_SERVER",
            Self::RegistryUsername => "REGISTRY_USERNAME",
            Self::RegistryPassword => "REGISTRY_PASSWORD",
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
