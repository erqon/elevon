use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{OpenOptions, create_dir_all},
    io::Result,
    path::{Path, PathBuf},
};

pub fn get_elevon_data_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").expect("HOME not set");
            PathBuf::from(home).join(".local/share")
        });

    let path = base.join("elevon");
    create_dir_all(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
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
    std::fs::write("/etc/systemd/system/elevon-agent-proxy.service", unit)
}

pub fn install_api_unit(exec: &Path) -> Result<()> {
    let unit = get_api_systemd_content(&exec.display().to_string());
    std::fs::write("/etc/systemd/system/elevon-agent-api.service", unit)
}
