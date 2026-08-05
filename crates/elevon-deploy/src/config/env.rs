use elevon_config::{ConfigError, ResolveEnv, deserialize_string_map, resolve_env_or_literal};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct EnvConfig {
    #[serde(default, deserialize_with = "deserialize_string_map")]
    pub vars: HashMap<String, String>,

    #[serde(default)]
    pub inherit: Vec<String>,

    #[serde(skip)]
    resolved: HashMap<String, String>,
}

impl ResolveEnv for EnvConfig {
    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        let mut resolved = HashMap::new();

        for (key, value) in std::mem::take(&mut self.vars) {
            resolved.insert(key, resolve_env_or_literal(&value)?);
        }

        for name in std::mem::take(&mut self.inherit) {
            let value =
                std::env::var(&name).map_err(|_| ConfigError::MissingEnv { name: name.clone() })?;
            resolved.entry(name).or_insert(value);
        }

        self.resolved = resolved;
        Ok(())
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            inherit: vec![],
            resolved: HashMap::new(),
        }
    }
}

impl EnvConfig {
    pub fn resolved(&self) -> &HashMap<String, String> {
        &self.resolved
    }
}
