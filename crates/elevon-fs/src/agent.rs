
use std::collections::HashMap;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{OpenOptions, create_dir_all, set_permissions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok, Result};
use elevon_contracts::deploy::TlsType;

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

pub fn load_app_string_env(app_name: &str) -> Result<Vec<String>> {
    Ok(load_app_env(app_name, None)?
        .iter()
        .map(|(key, val)| format!("{}={}", key, val))
        .collect())
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
        User=root
        ExecStart={exec} proxy
        Restart=on-failure
        RuntimeDirectory=elevon-agent
        
        [Install]
        WantedBy=multi-user.target
        "
    )
}

pub fn get_api_systemd_content(exec: &str) -> String {
    let home = std::env::var("HOME").expect("HOME not set");
    let user = whoami::username().expect("User not found");

    format!(
        "
        [Unit]
        Description=Elevon Agent HTTP Control API
        After=network.target
        
        [Service]
        User={user}
        Environment=HOME={home}
        ExecStart={exec} api
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

pub fn get_tls_file(project_name: &str, t: TlsType, is_project: Option<bool>) -> Result<PathBuf> {
    let file_dir_name = match is_project {
        Some(_) => format!("projects/{project_name}"),
        None => format!("{project_name}"),
    };

    let file_name = match t {
        TlsType::Cert => "cert.pem",
        TlsType::Key => "key.pem",
    };

    let path = AgentPath::ProjectTlsDir(format!("{file_dir_name}/{file_name}")).ensure()?;

    Ok(path)
}

pub fn write_tls_file(
    project_name: &str,
    content: &[u8],
    t: TlsType,
    is_project: Option<bool>,
) -> Result<()> {
    let path = get_tls_file(project_name, t, is_project)?;
    std::fs::write(path, content)?;
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
    ProjectTlsDir(String),
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

            AgentPath::EnvDir => base.join("etc").join("elevon").join("env"),

            AgentPath::AppEnv(app, bypass_default) => {
                if app == "default" && !*bypass_default && dev_root().is_none() {
                    anyhow::bail!("App can't be named 'default'")
                }
                AgentPath::EnvDir.resolve()?.join(format!("{app}.env"))
            }

            AgentPath::ProjectTlsDir(project) => {
                base.join("etc").join("elevon").join("tls").join(project)
            }

            AgentPath::Socket => base.join("run").join("elevon-agent.sock"),

            AgentPath::SystemdUnit(name) => {
                base.join("etc").join("systemd").join("system").join(name)
            }
        };

        Ok(p)
    }

    /// Ensure parent dirs exist.
    pub fn ensure(&self) -> Result<PathBuf> {
        let p = self.resolve()?;
        if let Some(parent) = p.parent() {
            create_dir_all(parent)?;
        }
        Ok(p)
    }

    pub fn ensure_dir(&self) -> Result<PathBuf> {
        let p = self.resolve()?;
        create_dir_all(p.clone())?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn resolve_base_path(is_prod: bool) -> PathBuf {
        match is_prod {
            true => PathBuf::from("/"),
            false => PathBuf::from("./devroot"),
        }
    }

    fn get_systemd_unit_path(base: PathBuf, name: String) -> PathBuf {
        base.join("etc").join("systemd").join("system").join(name)
    }

    #[test]
    fn test_base_path() {
        let dev_base = resolve_base_path(false);
        let prod_base = resolve_base_path(true);

        let dev_systemd_unit_path = get_systemd_unit_path(dev_base, "test".to_string());
        let prod_systemd_unit_path = get_systemd_unit_path(prod_base, "test".to_string());

        assert_eq!(
            dev_systemd_unit_path,
            PathBuf::from("./devroot/etc/systemd/system/test")
        );
        assert_eq!(
            prod_systemd_unit_path,
            PathBuf::from("/etc/systemd/system/test")
        );
    }
}
