use std::collections::HashMap;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{OpenOptions, create_dir_all, set_permissions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub fn get_elevon_data_path() -> Result<PathBuf> {
    let path = AgentPath::Data.ensure()?;
    if dev_root().is_none() {
        set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

pub fn get_database_path() -> Result<PathBuf> {
    let db_path = AgentPath::Database.ensure()?;

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&db_path)?;

    Ok(db_path)
}

pub fn get_env_path() -> Result<PathBuf> {
    let p = AgentPath::EnvDir.ensure()?;
    if dev_root().is_none() {
        set_permissions(&p, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(p)
}

pub fn get_app_env(app_name: &str, bypass_default: Option<bool>) -> Result<PathBuf> {
    let bypass = bypass_default.unwrap_or(false);
    if app_name == "default" && !bypass && dev_root().is_none() {
        anyhow::bail!("App can't be named 'default'");
    }

    let path = AgentPath::AppEnv(app_name.to_string(), bypass).ensure()?;
    if !path.exists() {
        std::fs::File::create(&path)?;
    }
    if dev_root().is_none() {
        set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
}

pub fn load_app_env(
    app_name: &str,
    bypass_default: Option<bool>,
) -> Result<HashMap<String, String>> {
    let app_path = get_app_env(app_name, bypass_default)?;

    let mut env = HashMap::new();
    env.extend(read_env_file(app_path)?);

    Ok(env)
}

pub fn add_app_env(
    app_name: &str,
    bypass_default: Option<bool>,
    key: String,
    value: String,
) -> Result<()> {
    let mut env = load_app_env(app_name, bypass_default)?;

    env.insert(key, value);

    let path = get_app_env(app_name, None).context("failed to resolve env file path")?;
    write_env_file(&path, &env)
        .with_context(|| format!("failed to write env file {}", path.display()))?;

    Ok(())
}

pub fn get_socket_path(delete: bool) -> PathBuf {
    let path = AgentPath::Socket
        .ensure()
        .unwrap_or_else(|_| PathBuf::from("/run/elevon-agent.sock"));
    if delete && path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    path
}

pub fn get_proxy_systemd_content(exec: &str) -> String {
    format!(
        "
        [Unit]
        Description=Elevon Agent Proxy
        After=network.target
        Wants=elevon-agent-api.service
        
        [Service]
        ExecStart={exec} proxy
        Restart=on-failure
        RuntimeDirectory=elevon-agent
        
        [Install]
        WantedBy=multi-user.target
        "
    )
}

pub fn get_api_systemd_content(exec: &str) -> String {
    format!(
        "
        [Unit]
        Description=Elevon Agent HTTP Control API
        After=network.target
        
        [Service]
        ExecStart={exec} server
        Restart=on-failure
        
        [Install]
        WantedBy=multi-user.target
        "
    )
}

pub fn install_proxy_unit(exec: &Path) -> Result<()> {
    let unit = get_proxy_systemd_content(&exec.display().to_string());
    let dest = AgentPath::SystemdUnit("elevon-agent-proxy.service".into()).ensure()?;
    std::fs::write(dest, unit)?;
    Ok(())
}

pub fn install_api_unit(exec: &Path) -> Result<()> {
    let unit = get_api_systemd_content(&exec.display().to_string());
    let dest = AgentPath::SystemdUnit("elevon-agent-api.service".into()).ensure()?;
    std::fs::write(dest, unit)?;
    Ok(())
}

fn read_env_file(path: impl AsRef<Path>) -> Result<HashMap<String, String>> {
    let mut vars = HashMap::new();

    for item in dotenvy::from_path_iter(path)? {
        let (key, value) = item?;
        vars.insert(key, value);
    }

    Ok(vars)
}

fn write_env_file(path: impl AsRef<Path>, env: &HashMap<String, String>) -> Result<()> {
    let mut contents = String::new();

    for (key, value) in env {
        contents.push_str(&format!("{key}={value}\n"));
    }

    std::fs::write(path, contents)?;
    Ok(())
}

fn dev_root() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        Some(PathBuf::from("./devroot"))
    } else {
        None
    }
}

pub enum AgentPath {
    Data,
    Database,
    EnvDir,
    AppEnv(String, bool),
    Socket,
    SystemdUnit(String),
}

impl AgentPath {
    pub fn resolve(&self) -> Result<PathBuf> {
        let base = match dev_root() {
            Some(root) => root,
            None => PathBuf::from("/"),
        };

        let p = match self {
            AgentPath::Data => {
                if dev_root().is_some() {
                    base.join("var").join("lib").join("elevon")
                } else {
                    // XDG_DATA_HOME logic kept as before
                    let base = std::env::var_os("XDG_DATA_HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            let home = std::env::var_os("HOME").expect("HOME not set");
                            PathBuf::from(home).join(".local/share")
                        });
                    base.join("elevon")
                }
            }

            AgentPath::Database => {
                let data = AgentPath::Data.resolve()?;
                data.join("elevon.db")
            }

            AgentPath::EnvDir => {
                if dev_root().is_some() {
                    base.join("etc").join("elevon").join("env")
                } else {
                    PathBuf::from("/etc").join("elevon").join("env")
                }
            }

            AgentPath::AppEnv(app, bypass_default) => {
                if app == "default" && !*bypass_default && dev_root().is_none() {
                    anyhow::bail!("App can't be named 'default'")
                }
                AgentPath::EnvDir.resolve()?.join(format!("{app}.env"))
            }

            AgentPath::Socket => {
                if dev_root().is_some() {
                    base.join("run").join("elevon-agent.sock")
                } else {
                    PathBuf::from("/run").join("elevon-agent.sock")
                }
            }

            AgentPath::SystemdUnit(name) => {
                if dev_root().is_some() {
                    base.join("etc").join("systemd").join("system").join(name)
                } else {
                    PathBuf::from("/etc")
                        .join("systemd")
                        .join("system")
                        .join(name)
                }
            }
        };

        Ok(p)
    }

    /// Ensure parent dirs exist and optionally set perms.
    pub fn ensure(&self) -> Result<PathBuf> {
        let p = self.resolve()?;
        if let Some(parent) = p.parent() {
            create_dir_all(parent)?;
        }
        Ok(p)
    }
}
