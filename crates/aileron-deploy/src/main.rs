use aileron_config::AileronConfig;
use aileron_deploy::config::Config;

fn main() {
    let config = Config::from_file("./config.yml").unwrap();

    println!("{config:#?}");
}
