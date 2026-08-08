use std::collections::HashMap;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{OpenOptions, create_dir_all, set_permissions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub fn get_elevon_data_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local/share")
        });

    let path = base.join("elevon");
    create_dir_all(&path)?;
    set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    Ok(path)
}

pub fn get_database_path() -> Result<PathBuf> {
    let db_path = get_elevon_data_path()?.join("elevon.db");

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&db_path)?;

    Ok(db_path)
}

pub fn get_env_path() -> Result<PathBuf> {
    let base = PathBuf::from("/etc/elevon/env");
    if !base.exists() {
        create_dir_all(&base)?;
    }
    set_permissions(&base, std::fs::Permissions::from_mode(0o700))?;

    Ok(base)
}

pub fn get_app_env(app_name: &str, bypass_default: Option<bool>) -> Result<PathBuf> {
    let bypass = bypass_default.unwrap_or(false);
    let base = get_env_path()?;

    if app_name == "default" && !bypass {
        anyhow::bail!("App can't be named 'default'");
    }

    let path = base.join(format!("{app_name}.env"));
    if !path.exists() {
        std::fs::File::create(&path)?;
    }
    set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;

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

pub fn get_socket_path() -> PathBuf {
    let path = PathBuf::from("/run/elevon-agent.sock");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
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
    std::fs::write("/etc/systemd/system/elevon-agent-proxy.service", unit)?;
    Ok(())
}

pub fn install_api_unit(exec: &Path) -> Result<()> {
    let unit = get_api_systemd_content(&exec.display().to_string());
    std::fs::write("/etc/systemd/system/elevon-agent-api.service", unit)?;
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
