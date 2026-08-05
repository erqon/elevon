use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    pub domain: String,
    pub port: u16,
}
