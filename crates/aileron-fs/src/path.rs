use std::path::PathBuf;

pub fn get_aileron_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").expect("HOME not set");
            PathBuf::from(home).join(".config")
        });

    let path = base.join("aileron");
    std::fs::create_dir_all(&path).expect("failed to create aileron configuration directory");
    path
}
