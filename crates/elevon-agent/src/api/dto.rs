use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppDeployData {
    pub name: String,
    pub image_url: String,
    pub domain: String,
    pub port: u16,
}
