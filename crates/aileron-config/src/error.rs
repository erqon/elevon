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
