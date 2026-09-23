use bollard::plugin::RestartPolicyNameEnum;
use bytesize::ByteSize;
use elevon_contracts::deploy::{AppRole, TlsConfig};
use serde::{Deserialize, Deserializer};

use crate::config::env::EnvConfig;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub role: AppRole,

    pub cmd: Option<String>,

    pub runtime: Option<AppRuntimeConfig>,

    pub env: Option<EnvConfig>,

    #[serde(skip)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AppRuntimeConfig {
    pub restart: RestartPolicyNameEnum,

    #[serde(alias = "cpu_limit")]
    pub cpu: Option<CpuLimit>,

    #[serde(alias = "memory_limit")]
    pub memory: Option<MemoryLimit>,

    pub network: Option<String>,
}

impl Default for AppRuntimeConfig {
    fn default() -> Self {
        Self {
            restart: RestartPolicyNameEnum::UNLESS_STOPPED,
            cpu: None,
            memory: None,
            network: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuLimit(pub f64);

impl CpuLimit {
    /// Converts core count into Docker nanoCPUs (i64)
    pub fn nano_cpus(&self) -> i64 {
        (self.0 * 1_000_000_000.0) as i64
    }
}

impl<'de> Deserialize<'de> for CpuLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Float(f64),
            Str(String),
        }

        match Value::deserialize(deserializer)? {
            Value::Float(f) => Ok(CpuLimit(f)),
            Value::Str(s) => {
                let parsed: f64 = s.trim().parse().map_err(serde::de::Error::custom)?;
                Ok(CpuLimit(parsed))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLimit(pub i64);

impl MemoryLimit {
    pub fn bytes(&self) -> i64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for MemoryLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Num(i64),
            Str(String),
        }

        match Value::deserialize(deserializer)? {
            Value::Num(n) => Ok(MemoryLimit(n)),
            Value::Str(s) => {
                let bytes = s
                    .parse::<ByteSize>()
                    .map_err(serde::de::Error::custom)?
                    .as_u64();
                Ok(MemoryLimit(bytes as i64))
            }
        }
    }
}
