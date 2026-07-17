use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub key: String,
}
