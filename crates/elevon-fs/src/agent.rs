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

#[derive(Clone)]
pub struct AppEnvOptions {
    pub project: String,
    pub app: Option<String>,
    pub deployment_id: Option<String>,
    pub bypass_default: Option<bool>,
}

impl AppEnvOptions {
    pub fn elevon() -> Self {
        Self {
            project: "default".to_string(),
            app: None,
            deployment_id: None,
            bypass_default: Some(true),
        }
    }

    pub fn app_base(project: &str, app: &str) -> Self {
        Self {
            project: project.to_string(),
            app: Some(app.to_string()),
            deployment_id: None,
            bypass_default: None,
        }
    }

    pub fn app(project: &str, app: &str, deployment_id: &str) -> Self {
        Self {
            project: project.to_string(),
            app: Some(app.to_string()),
            deployment_id: Some(deployment_id.to_string()),
            bypass_default: None,
        }
    }
}

pub fn get_app_env(options: AppEnvOptions) -> Result<PathBuf> {
    let bypass = options.bypass_default.unwrap_or(false);
    if options.project == "default" && !bypass && dev_root().is_none() {
        anyhow::bail!("App can't be named 'default'");
    }

    let path =
        AgentPath::AppEnv(options.project, options.app, options.deployment_id, bypass).ensure()?;
    if !path.exists() {
        std::fs::File::create(&path)?;
    }
    if dev_root().is_none() {
        set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
}

pub fn load_app_env(options: AppEnvOptions) -> Result<HashMap<String, String>> {
    let app_path = get_app_env(options)?;

    let mut env = HashMap::new();
    env.extend(read_env_file(app_path)?);

    Ok(env)
}

pub fn load_app_string_env(options: AppEnvOptions) -> Result<Vec<String>> {
    Ok(load_app_env(options)?
        .iter()
        .map(|(key, val)| format!("{}={}", key, val))
        .collect())
}

pub fn add_app_env(key: String, value: String, options: AppEnvOptions) -> Result<()> {
    let mut env = load_app_env(options.clone())?;

    env.insert(key, value);

    let path = get_app_env(options).context("failed to resolve env file path")?;
    write_env_file(&path, &env)
        .with_context(|| format!("failed to write env file {}", path.display()))?;

    Ok(())
}

#[derive(Clone)]
pub struct TlsOptions {
    pub project: String,
    pub app: Option<String>,
}

pub fn get_tls_file(ty: TlsType, options: TlsOptions) -> Result<PathBuf> {
    let file_dir_name = match options.app {
        Some(app) => format!("projects/{}/{app}", &options.project),
        None => options.project,
    };

    let path = AgentPath::ProjectTlsDir(format!("{file_dir_name}/{}", ty.get_file())).ensure()?;

    Ok(path)
}

pub fn write_tls_file(content: &[u8], ty: TlsType, options: TlsOptions) -> Result<()> {
    let path = get_tls_file(ty, options)?;
    std::fs::write(path, content)?;
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
    AppEnv(String, Option<String>, Option<String>, bool),
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

            AgentPath::AppEnv(project, app, deployment_id, bypass_default) => {
                if project == "default" && !*bypass_default && dev_root().is_none() {
                    anyhow::bail!("App can't be named 'default'")
                }

                let base = AgentPath::EnvDir.resolve()?;
                let mut path = base.join(project);

                if let Some(app) = app {
                    path = path.join(app);
                }

                if let Some(deployment_id) = deployment_id {
                    path = path.join(format!("dep-{deployment_id}"));
                }

                path.set_extension("env");
                path
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
