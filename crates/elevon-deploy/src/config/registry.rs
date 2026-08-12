use std::collections::HashMap;

use elevon_config::{ConfigError, ResolveEnvCredentials, resolve_env_or_literal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryConfig {
    pub server: String,
    pub username: String,
    pub password: String,
}

impl RegistryConfig {
    pub fn vars(&self) -> HashMap<String, String> {
        HashMap::from([
            ("REGISTRY_SERVER".to_string(), self.server.clone()),
            ("REGISTRY_USERNAME".to_string(), self.username.clone()),
            ("REGISTRY_PASSWORD".to_string(), self.password.clone()),
        ])
    }
}

impl ResolveEnvCredentials for RegistryConfig {
    type Output = Self;

    fn resolved_credentials(&self) -> Result<Self::Output, ConfigError> {
        Ok(Self {
            server: self.server.clone(),
            username: resolve_env_or_literal(&self.username)?,
            password: resolve_env_or_literal(&self.password)?,
        })
    }
}
