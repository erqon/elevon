use anyhow::Result;
use elevon_fs::agent::{add_app_env, load_app_env};

const ENV_PREFIX: &'static str = "ELEVON_";

#[derive(Debug)]
pub struct ElevonEnv {
    pub turso_remote_url: Option<String>,
}

impl ElevonEnv {
    pub fn load() -> Result<Self> {
        let vars = load_app_env("default")?;
        Ok(Self {
            turso_remote_url: vars.get(&ElevonEnvKey::TursoRemoteUrl.env_name()).cloned(),
        })
    }

    pub fn update_value(&mut self, key: ElevonEnvKey, value: impl Into<String>) -> Result<()> {
        match key {
            ElevonEnvKey::TursoRemoteUrl => {
                self.turso_remote_url = Some(value.into());
                add_app_env(
                    "default",
                    ElevonEnvKey::TursoRemoteUrl.env_name(),
                    self.turso_remote_url.clone().unwrap_or_default(),
                )?;
            }
        }
        Ok(())
    }
}

pub enum ElevonEnvKey {
    TursoRemoteUrl,
}

impl ElevonEnvKey {
    pub fn bare_name(&self) -> &'static str {
        match self {
            ElevonEnvKey::TursoRemoteUrl => "TURSO_REMOTE_URL",
        }
    }

    pub fn env_name(&self) -> String {
        format!("{ENV_PREFIX}{}", self.bare_name())
    }
}
