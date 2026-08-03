use elevon_config::ElevonConfig;
use elevon_deploy::config::Config;

fn main() {
    let config = Config::from_file("./config.yml").unwrap();

    println!("{config:#?}");
}
