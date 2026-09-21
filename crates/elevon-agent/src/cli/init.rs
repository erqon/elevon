use std::path::PathBuf;

use anyhow::Result;

pub fn run_init() -> Result<()> {
    let path = PathBuf::from("elevon-agent.yml");
    if path.exists() {
        eprintln!("elevon-agent.yml file already exists");
    }

    let template_content = include_str!("../templates/config.yml");
    std::fs::write(&path, template_content)?;

    println!("Config file was initialized {:?}", path);

    Ok(())
}
