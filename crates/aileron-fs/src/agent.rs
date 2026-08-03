use std::{
    io::Result,
    path::{Path, PathBuf},
};

pub fn get_socket_path() -> PathBuf {
    let path = PathBuf::from("/run/aileron-agent.sock");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    path
}

pub fn get_proxy_systemd_content(exec: &str) -> String {
    format!(
        "
        [Unit]
        Description=Aileron Agent Proxy
        After=network.target
        Wants=aileron-agent-server.service
        
        [Service]
        ExecStart={exec} proxy
        Restart=on-failure
        RuntimeDirectory=aileron-agent
        
        [Install]
        WantedBy=multi-user.target
        "
    )
}

pub fn get_api_systemd_content(exec: &str) -> String {
    format!(
        "
        [Unit]
        Description=Aileron Agent HTTP Control API
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
    std::fs::write("/etc/systemd/system/aileron-agent-proxy.service", unit)
}

pub fn install_api_unit(exec: &Path) -> Result<()> {
    let unit = get_api_systemd_content(&exec.display().to_string());
    std::fs::write("/etc/systemd/system/aileron-agent-api.service", unit)
}
