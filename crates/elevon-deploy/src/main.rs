use elevon_config::ElevonConfig;
use elevon_deploy::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::from_file("./config.yml")?;
    println!("{config:#?}");
    Ok(())
}
