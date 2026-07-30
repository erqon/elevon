mod error;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::Path;

pub use error::ConfigError;

pub trait ResolveEnv {
    /// Resolve `$VAR` fields. Override in each config crate.
    fn resolve_env(&mut self) -> Result<(), ConfigError>;
}

pub trait AileronConfig: ResolveEnv + DeserializeOwned + Sized
where
    Self: 'static,
{
    fn from_str(yaml: &str) -> Result<Self, ConfigError> {
        let mut config: Self = serde_yml::from_str(yaml)?;
        config.resolve_env()?;
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

pub fn resolve_env_or_literal(value: String) -> Result<String, ConfigError> {
    if let Some(name) = value.strip_prefix('$') {
        std::env::var(name).map_err(|_| ConfigError::MissingEnv {
            name: name.to_string(),
        })
    } else {
        Ok(value)
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
