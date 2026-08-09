mod error;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::Path;

pub use error::ConfigError;

/// Expand `$VAR` fields in place.
pub trait ResolveEnv {
    fn resolve_env(&mut self) -> Result<(), ConfigError>;
}

/// Resolve `$VAR`/env refs into a new map; config is left unchanged.
pub trait ResolveEnvCredentials {
    type Output;
    fn resolved_credentials(&self) -> Result<Self::Output, ConfigError>;
}

pub trait ElevonConfig: DeserializeOwned + Sized
where
    Self: 'static,
{
    fn from_str(yaml: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_yml::from_str(yaml)?;
        Ok(config)
    }

    fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_str(&text)
    }
}

pub fn resolve_env_or_literal(value: &str) -> Result<String, ConfigError> {
    if let Some(rest) = value.strip_prefix('$') {
        // Check for $VAR:-default syntax
        if let Some((name, default)) = rest.split_once(":-") {
            return Ok(std::env::var(name).unwrap_or_else(|_| default.to_string()));
        }
        // Plain $VAR (no default, must be set)
        std::env::var(rest).map_err(|_| ConfigError::MissingEnv {
            name: rest.to_string(),
        })
    } else {
        // Literal value
        Ok(value.to_string())
    }
}

pub fn deserialize_string_map<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, serde_yml::Value> = HashMap::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(k, v)| {
            let s = match v {
                serde_yml::Value::String(s) => s,
                serde_yml::Value::Number(n) => n.to_string(),
                serde_yml::Value::Bool(b) => b.to_string(),
                serde_yml::Value::Null => String::new(),
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "env var `{k}` must be a scalar, got {other:?}"
                    )));
                }
            };
            Ok((k, s))
        })
        .collect()
}
