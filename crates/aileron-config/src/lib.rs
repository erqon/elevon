use serde::Deserialize;
use std::path::Path;

pub use database::{DatabaseConfig, PostgresConfig, TursoConfig, Type as DatabaseType};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file `{path}`")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config")]
    Parse(#[from] serde_yml::Error),
    #[error("environment variable `{name}` is not set")]
    MissingEnv { name: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_str(&text)
    }

    pub fn from_str(yaml: &str) -> Result<Self, ConfigError> {
        let mut config: Config = serde_yml::from_str(yaml)?;
        config.resolve_env()?;
        Ok(config)
    }

    fn resolve_env(&mut self) -> Result<(), ConfigError> {
        match &mut self.database {
            DatabaseConfig::Turso(cfg) => {
                cfg.path = resolve_env_or_literal(std::mem::take(&mut cfg.path))?;
            }
            DatabaseConfig::Postgres(cfg) => {
                cfg.url = resolve_env_or_literal(std::mem::take(&mut cfg.url))?;
            }
        }
        Ok(())
    }
}

pub fn load() -> Result<Config, ConfigError> {
    Config::from_file("aileron.yml")
}

fn resolve_env_or_literal(value: String) -> Result<String, ConfigError> {
    if let Some(name) = value.strip_prefix('$') {
        std::env::var(name).map_err(|_| ConfigError::MissingEnv {
            name: name.to_string(),
        })
    } else {
        Ok(value)
    }
}

mod database {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum DatabaseConfig {
        Turso(TursoConfig),
        Postgres(PostgresConfig),
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct TursoConfig {
        pub path: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PostgresConfig {
        pub url: String,
        pub max_connections: Option<u32>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Type {
        Turso,
        Postgres,
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_from_str() -> Result<(), ConfigError> {
        let yaml = r#"
            database:
                type: turso
                path: ./data.db
        "#;
        let config = Config::from_str(yaml)?;
        assert!(matches!(config.database, DatabaseConfig::Turso(_)));
        Ok(())
    }

    #[test]
    fn test_env_value() -> Result<(), ConfigError> {
        const VAR: &str = "AILERON_TEST_TURSO_PATH";
        unsafe {
            std::env::set_var(VAR, "/tmp/test.db");
        }

        let yaml = format!(
            r#"
            database:
                type: turso
                path: ${VAR}
        "#
        );

        let config = Config::from_str(&yaml)?;
        assert!(matches!(
            config.database,
            DatabaseConfig::Turso(TursoConfig { path }) if path == "/tmp/test.db"
        ));

        unsafe {
            std::env::remove_var(VAR);
        }
        Ok(())
    }

    #[test]
    fn test_missing_env() {
        const VAR: &str = "AILERON_TEST_MISSING_ENV";
        unsafe {
            std::env::remove_var(VAR);
        }

        let yaml = format!(
            r#"
            database:
                type: turso
                path: ${VAR}
        "#
        );

        let err = Config::from_str(&yaml).unwrap_err();
        assert!(matches!(err, ConfigError::MissingEnv { name } if name == VAR));
    }
}
