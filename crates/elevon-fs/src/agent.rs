use std::{
    io::Result,
    path::{Path, PathBuf},
};

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
