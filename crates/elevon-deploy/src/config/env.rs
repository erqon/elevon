use elevon_config::{
    ConfigError, ResolveEnvCredentials, deserialize_string_map, resolve_env_or_literal,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnvConfig {
    #[serde(default, deserialize_with = "deserialize_string_map")]
    pub vars: HashMap<String, String>,

    #[serde(default)]
    pub inherit: Vec<String>,
}

impl ResolveEnvCredentials for EnvConfig {
    type Output = HashMap<String, String>;

    fn resolved_credentials(&self) -> Result<Self::Output, ConfigError> {
        let mut resolved = HashMap::new();

        for (key, value) in &self.vars {
            resolved.insert(key.clone(), resolve_env_or_literal(value)?);
        }

        for name in &self.inherit {
            let value =
                std::env::var(name).map_err(|_| ConfigError::MissingEnv { name: name.clone() })?;
            resolved.entry(name.clone()).or_insert(value);
        }

        Ok(resolved)
    }
}
